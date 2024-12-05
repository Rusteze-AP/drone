// use crate::messages::{RustezePacket, RustezeSourceRoutingHeader};
use crossbeam::channel::{select, Receiver, Sender};
use logger::Logger;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::drone::Drone;
use wg_internal::network::{NodeId, SourceRoutingHeader};
use wg_internal::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType,
};

pub struct RustezeDrone {
    id: NodeId,
    pdr: f32,
    packet_senders: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    terminated: bool,

    // Flood related
    flood_history: HashSet<u64>,
    logger: Logger,
}

impl Drone for RustezeDrone {
    fn new(
        id: NodeId,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        pdr: f32,
    ) -> Self {
        Self {
            id,
            pdr,
            packet_senders: packet_send,
            packet_recv,
            controller_send,
            controller_recv,
            terminated: false,
            flood_history: HashSet::new(),
            logger: Logger::new(true, "RustezeDrone".to_string()),
        }
    }

    fn run(&mut self) {
        self.internal_run();
    }
}

#[derive(PartialEq, Copy, Clone)]
enum Format {
    LowerCase,
    UpperCase,
}

impl RustezeDrone {
    /// Return the `NodeId` of the Drone
    pub fn get_id(&self) -> NodeId {
        self.id
    }

    fn get_packet_type(pt: &PacketType, format: Format) -> String {
        if format == Format::LowerCase {
            match pt {
                PacketType::Ack(_) => "Ack".to_string(),
                PacketType::Nack(_) => "Nack".to_string(),
                PacketType::FloodRequest(_) => "Flood request".to_string(),
                PacketType::FloodResponse(_) => "Flood response".to_string(),
                PacketType::MsgFragment(_) => "Fragment".to_string(),
            }
        } else {
            match pt {
                PacketType::Ack(_) => "ACK".to_string(),
                PacketType::Nack(_) => "NACK".to_string(),
                PacketType::FloodRequest(_) => "FLOOD REQUEST".to_string(),
                PacketType::FloodResponse(_) => "FLOOD REPONSE".to_string(),
                PacketType::MsgFragment(_) => "FRAGMENT".to_string(),
            }
        }
    }

    fn check_next_hop(&mut self, current_node: NodeId, packet: &mut Packet) -> Result<(), String> {
        // If current_node is wrong
        if current_node != self.id {
            let packet_capital = Self::get_packet_type(&packet.pack_type, Format::UpperCase);
            if packet_capital == "FRAGMENT" {
                // TODO - Send NACK - UnexpectedRecipient(self.id)
            }
            return Err(format!("[DRONE-{}][NACK] - {} received by the wrong Node. Found DRONE {} at current hop. Ignoring!", self.id, packet_capital, current_node));
        }

        // If current_node is correct
        packet.routing_header.increase_hop_index();
        match packet.routing_header.current_hop() {
            Some(next_node) => Ok(()),
            None => {
                // TODO - Send NACK - UnexpectedRecipient(self.id)
                Err(format!("[DRONE-{}][NACK] - No next hop found", self.id))
            }
        }
    }

    fn generic_packet_check(&mut self, packet: &mut Packet) -> Result<(), String> {
        if let Some(current_node) = packet.routing_header.current_hop() {
            self.check_next_hop(current_node, packet)
        } else {
            let pt = Self::get_packet_type(&packet.pack_type, Format::UpperCase);
            if pt == "FRAGMENT" {
                // TODO - Send NACK - UnexpectedRecipient(self.id)
            }
            Err(format!("[DRONE-{}][NACK] - No current hop found", self.id))
        }
    }

    fn packet_dispatcher(&mut self, mut packet: Packet) {
        // Check if header is valid
        if let Err(err) = self.generic_packet_check(&mut packet) {
            self.logger.log_error(err.as_str());
            return;
        }

        // TODO - Handle different packet types (fn call)
    }

    fn internal_run(&mut self) {
        loop {
            if self.terminated {
                self.logger
                    .log_info(format!("[DRONE-{}][RUNNER] - Terminated", self.id).as_str());
                break;
            }
            select! {
                recv(self.packet_recv) -> msg => {
                    if let Ok(msg) = msg { self.packet_dispatcher(msg) } else {
                        self.logger.log_error(format!("[DRONE-{}][RUNNER] - Drone receiver disconnected. Terminating thread...", self.id).as_str());
                        break;
                    }
                }
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.logger.log_info(format!("[Drone-{}][RUNNER] - Received message from controller", self.id).as_str());
                        // self.execute_command(command);
                    } else {
                        self.logger.log_error(format!("[DRONE-{}][RUNNER] - Simulation controller receiver disconnected. Terminating thread...", self.id).as_str());
                        break;
                    }
                }
            }
        }
    }
}
