use crate::log_debug;
use crate::messages::RustezePacket;
use crate::messages::RustezeSourceRoutingHeader;
use crossbeam::channel::{select, Receiver, Sender};
use std::collections::HashMap;
use wg_internal::controller::Command;
use wg_internal::drone::{Drone, DroneOptions};
use wg_internal::network::NodeId;
use wg_internal::network::SourceRoutingHeader;
use wg_internal::packet::Fragment;
use wg_internal::packet::Packet;
use wg_internal::packet::PacketType;

pub struct RustezeDrone {
    id: NodeId,
    receiver: Receiver<Packet>,
    senders: HashMap<NodeId, Sender<Packet>>,
    controller_receiver: Receiver<Command>,
    controller_sender: Sender<Command>,
}

impl Drone for RustezeDrone {
    fn new(options: DroneOptions) -> Self {
        Self {
            id: options.id,
            receiver: options.packet_recv,
            senders: HashMap::new(),
            controller_receiver: options.sim_contr_recv,
            controller_sender: options.sim_contr_send,
        }
    }

    fn run(&mut self) {
        self.internal_run();
    }
}

impl RustezeDrone {
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
                        Some(next_node) => match self.senders.get(&next_node) {
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

    pub fn internal_run(&mut self) {
        loop {
            select! {
                recv(self.receiver) -> msg => {
                    match msg {
                        Ok(msg) => self.packet_dispatcher(msg),
                        Err(_) => {
                            log_debug!("Drone {} receiver disconnected", self.id);
                            break;
                        }
                    }
                }
                recv(self.controller_receiver) -> msg => {
                    if let Ok(msg) = msg {
                        log_debug!("Drone {} received message from controller", self.id);
                    } else {
                        log_debug!("Drone {} controller receiver disconnected", self.id);
                        break;
                    }
                }
            }
        }
    }
}
