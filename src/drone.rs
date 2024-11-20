use std::collections::{HashMap};

use super::messages::*;
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

    fn drone_packet_dispatcher(&self, packet: Packet) {
        match packet.pack_type {
            PacketType::MsgFragment(fragment) => {
                println!("Drone {} received a fragment", self.id);
                
            }
            PacketType::Nack(nack) => {
                println!("Drone {} received nack", self.id);
            }
            PacketType::Ack(ack) => {
                println!("Drone {} received ack", self.id);
            }
        }
    }

    fn drone_fragment_handler(
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
                            println!("Drone {} received fragment and it is at the edge", self.id);
                        },
                        Some(next_node) => {
                            match self.senders.get(&next_node) {
                                None => {
                                    println!("Drone {} received fragment and can't forward", self.id);
                                },
                                Some(sender) => {
                                    let packet = Packet::new(PacketType::MsgFragment(fragment), source_routing_header, pusession_id);
                                    sender.send(packet).unwrap();
                                }
                            }
                        }
                    }
                    
                } else {
                    println!("Drone {} received wrong fragment", self.id);
                }
            }
            None => {
                println!("Drone {} received wrong fragment", self.id);
            }
        }
    }

    pub fn run(&self) {
        loop {
            select! {
                recv(self.receiver) -> msg => {
                    match msg {
                        Ok(msg) => self.drone_packet_dispatcher(msg),
                        Err(_) => {
                            println!("Drone {} receiver disconnected", self.id);
                            break;
                        }
                    }
                }
                recv(self.controller_receiver) -> msg => {
                    if let Ok(msg) = msg {
                        println!("Drone {} received message from controller", self.id);
                    } else {
                        println!("Drone {} controller receiver disconnected", self.id);
                        break;
                    }
                }
            }
        }
    }
}
