use super::messages::NodeId;
use crossbeam::channel::{select, Receiver, Sender};


pub struct Drone {
    id: NodeId,
    links: Vec<NodeId>,
    receiver: Receiver<String>,
    controller_receiver: Receiver<String>,
    sender: Vec<Sender<String>>,
    controller_sender: Sender<String>,
}

impl Drone {
    pub fn new(
        id: NodeId,
        links: Vec<NodeId>,
        receiver: Receiver<String>,
        controller_receiver: Receiver<String>,
        sender: Vec<Sender<String>>,
        controller_sender: Sender<String>,
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

    pub fn run(&self) {
        loop {
            select!{
                recv(self.receiver) -> msg => {
                    if let Ok(msg) = msg {
                        println!("Drone {} received message: {}", self.id, msg);
                    } else {
                        println!("Drone {} receiver disconnected", self.id);
                        break;
                    }
                }
                recv(self.controller_receiver) -> msg => {
                    if let Ok(msg) = msg {
                        println!("Drone {} received controller message: {}", self.id, msg);
                    } else {
                        println!("Drone {} controller receiver disconnected", self.id);
                        break;
                    }
                }
            }
        }
    }
}
