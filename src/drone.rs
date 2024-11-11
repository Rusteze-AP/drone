use super::messages::*;
use crossbeam::channel::{select, Receiver, Sender};


pub struct Drone {
    id: NodeId,
    links: Vec<NodeId>,
    receiver: Receiver<Packet>,
    controller_receiver: Receiver<Packet>,
    sender: Vec<Sender<Packet>>,
    controller_sender: Sender<Packet>,
}

impl Drone {
    pub fn new(
        id: NodeId,
        links: Vec<NodeId>,
        receiver: Receiver<Packet>,
        controller_receiver: Receiver<Packet>,
        sender: Vec<Sender<Packet>>,
        controller_sender: Sender<Packet>,
    ) -> Drone {
        Drone {
            id,
            links,
            receiver,
            controller_receiver,
            sender,
            controller_sender,
        }
    }
    
    fn drone_packet_dispatcher(&self, packet: Packet){
        match packet.pack_type {
            PacketType::MsgFragment(fragment) => {
                println!("Drone {} received a fragment", self.id);
            },
            PacketType::Nack(nack) => {
                println!("Drone {} received nack", self.id);
            },
            PacketType::Ack(ack) => {
                println!("Drone {} received ack", self.id);
            }
        }
    }

    pub fn run(&self) {
        loop {
            select!{
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
