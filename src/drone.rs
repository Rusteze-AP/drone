use std::collections::{HashMap};
use super::messages::*;
use crate::log_debug;
use crossbeam::channel::{select, Receiver, Sender};

pub struct Drone {
    id: NodeId,
    receiver: Receiver<Packet>,
    controller_receiver: Receiver<Packet>,
    senders: HashMap<NodeId, Sender<Packet>>,
    controller_sender: Sender<Packet>,
}

impl Drone {
    pub fn new(
        id: NodeId,
        receiver: Receiver<Packet>,
        controller_receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
        controller_sender: Sender<Packet>,
    ) -> Drone {
        Drone {
            id,
            receiver,
            controller_receiver,
            senders,
            controller_sender,
        }
    }

    fn packet_dispatcher(&self, packet: Packet) {
        match packet.pack_type {
            PacketType::MsgFragment(fragment) => {
                log_debug!("Drone {} received a fragment", self.id);
                // self.fragment_handler(fragment, packet.routing_header, packet.session_id);             
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
                        },
                        Some(next_node) => {
                            match self.senders.get(&next_node) {
                                None => {
                                    log_debug!("Drone {} received fragment and can't forward", self.id);
                                },
                                Some(sender) => {
                                    let packet = Packet::new(PacketType::MsgFragment(fragment), source_routing_header, pusession_id);
                                    sender.send(packet).unwrap();
                                }
                            }
                        }
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

    pub fn run(&self) {
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
