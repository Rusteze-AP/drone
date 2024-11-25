use crate::log_debug;
use crate::messages::{RustezeSourceRoutingHeader, RustezePacket};
use crossbeam::channel::{select, Receiver, Sender};
use std::collections::HashMap;
use wg_internal::controller::{NodeEvent, DroneCommand};
use wg_internal::drone::{Drone, DroneOptions};
use wg_internal::network::{NodeId,SourceRoutingHeader};
use wg_internal::packet::{Fragment, Packet, PacketType};

pub struct RustezeDrone {
    id: NodeId,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    controller_send: Sender<NodeEvent>,
    controller_recv: Receiver<DroneCommand>,
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

    fn packet_dispatcher(&mut self, packet: Packet) {
        match packet.pack_type {
            PacketType::MsgFragment(fragment) => {
                log_debug!("Drone {} received a fragment", self.id);
                self.fragment_handler(fragment, packet.routing_header, packet.session_id);
            }
            PacketType::Nack(nack) => {
                log_debug!("Drone {} received nack", self.id);
            }
            PacketType::Ack(ack) => {
                log_debug!("Drone {} received ack", self.id);
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
        pusession_id: u64,
    ) {
        match source_routing_header.get_next_hop() {
            Some(hop) => {
                if hop == self.id {
                    source_routing_header.increment_index();
                    match source_routing_header.get_next_hop() {
                        None => {
                            log_debug!("Drone {} received fragment and it is at the edge", self.id);
                        }
                        Some(next_node) => match self.packet_send.get(&next_node) {
                            None => {
                                log_debug!("Drone {} received fragment and can't forward", self.id);
                            }
                            Some(sender) => {
                                let packet = Packet::new(
                                    PacketType::MsgFragment(fragment),
                                    Box::new(source_routing_header),
                                    pusession_id,
                                );
                                sender.send(packet).unwrap();
                            }
                        },
                    }
                } else {
                    log_debug!("Drone {} received wrong fragment", self.id);
                }
            }
            None => {
                log_debug!("Drone {} received wrong fragment", self.id);
            }
        }
    }

    fn execute_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::AddSender(id, sender ) => {
                unimplemented!()
            },
            DroneCommand::SetPacketDropRate(pdr) => {
                unimplemented!()
            },
            DroneCommand::Crash => {
                unimplemented!()
            }
        }
    }

    pub fn internal_run(&mut self) {
        loop {
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
                    } else {
                        log_debug!("Drone {} controller receiver disconnected from Simulation Controller", self.id);
                        break;
                    }
                }
            }
        }
    }
}
