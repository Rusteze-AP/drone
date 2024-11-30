use crate::log_debug;
use crate::messages::{RustezePacket, RustezeSourceRoutingHeader};
use crossbeam::channel::{select, Receiver, Sender};
use rand::Rng;
use std::collections::HashMap;
use std::thread::current;
use wg_internal::controller::{DroneCommand, NodeEvent};
use wg_internal::drone::{Drone, DroneOptions};
use wg_internal::network::{NodeId, SourceRoutingHeader};
use wg_internal::packet::{Ack, Fragment, Nack, NackType, Packet, PacketType};

pub struct RustezeDrone {
    id: NodeId,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    controller_send: Sender<NodeEvent>,
    controller_recv: Receiver<DroneCommand>,
    terminated: bool,
}

impl Drone for RustezeDrone {
    fn new(options: DroneOptions) -> Self {
        Self {
            id: options.id,
            pdr: options.pdr,
            packet_send: options.packet_send,
            packet_recv: options.packet_recv,
            controller_send: options.controller_send,
            controller_recv: options.controller_recv,
            terminated: false,
        }
    }

    fn run(&mut self) {
        self.internal_run();
    }
}

impl RustezeDrone {
    /// Return the NodeId of the Drone
    pub fn get_id(&self) -> NodeId {
        self.id
    }

    fn packet_dispatcher(&mut self, mut packet: Packet) {
        match packet.pack_type {
            PacketType::MsgFragment(fragment) => {
                log_debug!(
                    "[DRONE-{}][🟢 FRAGMENT] - Received a fragment from NODE {}",
                    self.id,
                    packet.routing_header.get_previous_hop().unwrap_or(0)
                );
                self.fragment_handler(fragment, packet.routing_header, packet.session_id);
            }
            PacketType::Nack(nack) => {
                log_debug!(
                    "[DRONE-{}][🟢 NACK] - Received a nack from NODE {}",
                    self.id,
                    packet.routing_header.get_previous_hop().unwrap_or(0)
                );
                match packet.routing_header.get_current_hop() {
                    None => log_debug!("[DRONE-{}][🔴 NACK] - No current hop found", self.id),
                    Some(current_node) => {
                        if current_node == self.id {
                            packet.routing_header.increment_index();
                            self.send_nack(
                                nack.nack_type,
                                nack.fragment_index,
                                packet.session_id,
                                packet.routing_header,
                            );
                        } else {
                            log_debug!("[DRONE-{}][🔴 NACK] - Nack received by the wrong node. Found DRONE {} at current hop. Ignoring!", self.id, current_node);
                        }
                    }
                }
            }
            PacketType::Ack(ack) => {
                log_debug!(
                    "[DRONE-{}][🟢 ACK] - Received an ack from NODE {}",
                    self.id,
                    packet.routing_header.get_previous_hop().unwrap_or(0)
                );
                match packet.routing_header.get_current_hop() {
                    None => log_debug!("[DRONE-{}][🔴 ACK] - No current hop found", self.id),
                    Some(current_node) => {
                        if current_node == self.id {
                            packet.routing_header.increment_index();
                            self.send_ack(
                                ack.fragment_index,
                                packet.session_id,
                                packet.routing_header,
                            );
                        } else {
                            log_debug!("[DRONE-{}][🔴 ACK] - Ack received by the wrong Node. Found DRONE {} at current hop. Ignoring!", self.id, current_node);
                        }
                    }
                }
            }
            _ => {
                log_debug!("Drone {} received unknown packet", self.id);
            }
        }
    }

    fn fragment_handler(
        &mut self,
        fragment: Fragment,
        mut source_routing_header: SourceRoutingHeader,
        session_id: u64,
    ) {
        match source_routing_header.get_current_hop() {
            Some(hop) => {
                if hop == self.id {
                    source_routing_header.increment_index();
                    match source_routing_header.get_current_hop() {
                        None => {
                            log_debug!("Drone {} received fragment and it is at the edge", self.id);
                            self.send_nack(
                                NackType::DestinationIsDrone,
                                fragment.fragment_index,
                                session_id,
                                source_routing_header,
                            )
                        }
                        Some(next_node) => match self.packet_send.get(&next_node) {
                            None => {
                                log_debug!("Drone {} received fragment and can't forward", self.id);
                                self.send_nack(
                                    NackType::ErrorInRouting(next_node),
                                    fragment.fragment_index,
                                    session_id,
                                    source_routing_header,
                                )
                            }
                            Some(sender) => {
                                if self.to_drop() {
                                    // TODO Add log_debug!
                                    self.send_nack(
                                        NackType::Dropped,
                                        fragment.fragment_index,
                                        session_id,
                                        SourceRoutingHeader {
                                            hop_index: 1,
                                            hops: source_routing_header
                                                .hops
                                                .split_at(source_routing_header.hop_index)
                                                .0
                                                .iter()
                                                .rev()
                                                .cloned()
                                                .collect(),
                                        },
                                    );
                                } else {
                                    let packet = Packet::new(
                                        PacketType::MsgFragment(fragment.clone()),
                                        source_routing_header.clone(),
                                        session_id,
                                    );
                                    sender.send(packet).unwrap();
                                    self.send_ack(
                                        fragment.fragment_index,
                                        session_id,
                                        SourceRoutingHeader {
                                            hop_index: 1,
                                            hops: source_routing_header
                                                .hops
                                                .split_at(source_routing_header.hop_index)
                                                .0
                                                .iter()
                                                .rev()
                                                .cloned()
                                                .collect(),
                                        },
                                    );
                                }
                            }
                        },
                    }
                } else {
                    log_debug!("Drone {} is not the right fragment receiver", self.id);
                    source_routing_header.increment_index();
                    self.send_nack(
                        NackType::UnexpectedRecipient(self.id),
                        fragment.fragment_index,
                        session_id,
                        source_routing_header,
                    )
                }
            }
            None => {
                log_debug!("Drone {} is not the right fragment receiver", self.id);
                source_routing_header.increment_index();
                self.send_nack(
                    NackType::UnexpectedRecipient(self.id),
                    fragment.fragment_index,
                    session_id,
                    source_routing_header,
                )
            }
        }
    }

    fn execute_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::AddSender(id, sender) => {
                self.packet_send.insert(id, sender);
                log_debug!("Added new sender for node {}", id);
            }
            DroneCommand::SetPacketDropRate(pdr) => {
                self.pdr = pdr;
                log_debug!("Packet drop rate of node {} changed to {}", self.id, pdr);
            }
            DroneCommand::Crash => {
                // exit the thread
                log_debug!("Received crash command for node {}", self.id);
                self.terminated = true;
                // TODO Decide how to handle the crash (packets still in channel?)
            }
        }
    }

    fn send_nack(
        &mut self,
        nack_type: NackType,
        fragment_index: u64,
        session_id: u64,
        source_routing_header: SourceRoutingHeader,
    ) {
        let nack = Nack {
            fragment_index,
            nack_type,
        };
        match source_routing_header.get_current_hop() {
            None => log_debug!(
                "[🔴 NACK] - No next hop found. Hops {:?} with hop_index {}",
                source_routing_header.hops,
                source_routing_header.hop_index
            ),
            Some(previous_node) => match self.packet_send.get(&previous_node) {
                None => log_debug!(
                    "[🔴 NACK] - No match of NODE {} found inside neighbours {:?} at hop_index {}",
                    previous_node,
                    source_routing_header.hops,
                    source_routing_header.hop_index
                ),
                Some(sender) => {
                    let packet =
                        Packet::new(PacketType::Nack(nack), source_routing_header, session_id);
                    log_debug!(
                        "[DRONE-{}][🟢 NACK] - Ack forwarded to NODE {}",
                        self.id,
                        previous_node
                    );
                    sender.send(packet).unwrap();
                }
            },
        }
    }

    fn send_ack(
        &mut self,
        fragment_index: u64,
        session_id: u64,
        source_routing_header: SourceRoutingHeader,
    ) {
        match source_routing_header.get_current_hop() {
            None => log_debug!(
                "[🔴 ACK] - No next hop found. Hops {:?} with hop_index {}",
                source_routing_header.hops,
                source_routing_header.hop_index
            ),
            Some(previous_node) => match self.packet_send.get(&previous_node) {
                None => log_debug!(
                    "[🔴 ACK] - No match of NODE {} found inside neighbours {:?} at hop_index {}",
                    previous_node,
                    source_routing_header.hops,
                    source_routing_header.hop_index
                ),
                Some(sender) => {
                    let packet = Packet::new(
                        PacketType::Ack(Ack { fragment_index }),
                        source_routing_header,
                        session_id,
                    );
                    log_debug!(
                        "[DRONE-{}][🟢 ACK] - Ack forwarded to NODE {}",
                        self.id,
                        previous_node
                    );
                    sender.send(packet).unwrap();
                }
            },
        }
    }

    fn to_drop(&self) -> bool {
        let mut rng = rand::thread_rng();
        let random_value: f32 = rng.gen();
        self.pdr > random_value
    }

    fn internal_run(&mut self) {
        loop {
            if self.terminated {
                log_debug!("Drone {} terminated", self.id);
                break;
            }
            select! {
                recv(self.packet_recv) -> msg => {
                    match msg {
                        Ok(msg) => self.packet_dispatcher(msg),
                        Err(_) => {
                            log_debug!("Drone {} receiver disconnected", self.id);
                            break;
                        }
                    }
                }
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        log_debug!("Drone {} received message from controller", self.id);
                        self.execute_command(command);
                    } else {
                        log_debug!("Drone {} controller receiver disconnected from Simulation Controller", self.id);
                        break;
                    }
                }
            }
        }
    }
}
