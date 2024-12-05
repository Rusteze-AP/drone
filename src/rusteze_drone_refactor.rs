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

use crate::messages::RustezeSourceRoutingHeader;
use crate::packet_send::*;

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

    fn check_next_hop(
        &mut self,
        current_node: NodeId,
        packet: &mut Packet,
    ) -> Result<Sender<Packet>, (String, String)> {
        let packet_capital = Self::get_packet_type(&packet.pack_type, Format::UpperCase);
        let mut send_res = "".to_string();
        // If current_node is wrong
        if current_node != self.id {
            if packet_capital == "FRAGMENT" {
                // TODO - Send NACK - UnexpectedRecipient(self.id)
                // send_res = send_nack()
            }
            return Err((send_res, format!("[DRONE-{}][{}] - {} received by the wrong Node. Found DRONE {} at current hop. Ignoring!", self.id, packet_capital ,packet_capital, current_node)));
        }

        // If current_node is correct, increase hop index
        packet.routing_header.increase_hop_index();
        // Check if the new hop exists
        match packet.routing_header.current_hop() {
            Some(next_node) => {
                let sender_res = get_sender(next_node, &self.packet_senders);
                match sender_res {
                    Ok(sender) => Ok(sender),
                    Err(err) => {
                        if packet_capital == "FRAGMENT" {
                            // TODO - Send NACK - ErrorInRouting(next_node)
                            // send_res = send_nack()
                        }
                        Err((
                            send_res,
                            format!("[DRONE-{}][{}] - {err}", self.id, packet_capital),
                        ))
                    }
                }
            }
            None => {
                if packet_capital == "FRAGMENT" {
                    // TODO - Send NACK - UnexpectedRecipient(self.id)
                    // send_res = send_nack()
                }
                Err((
                    send_res,
                    format!("[DRONE-{}][NACK] - No next hop found", self.id),
                ))
            }
        }
    }

    fn generic_packet_check(
        &mut self,
        packet: &mut Packet,
    ) -> Result<Sender<Packet>, (String, String)> {
        if let Some(current_node) = packet.routing_header.current_hop() {
            return self.check_next_hop(current_node, packet);
        } else {
            let pt = Self::get_packet_type(&packet.pack_type, Format::UpperCase);
            let mut res = (
                "".to_string(),
                format!("[DRONE-{}][NACK] - No current hop found", self.id),
            );
            if pt == "FRAGMENT" {
                // TODO - Send NACK - UnexpectedRecipient(self.id)
                // res.0 = send_nack()
            }
            Err(res)
        }
    }

    fn packet_dispatcher(&mut self, mut packet: Packet) {
        // Check if header is valid
        let sender = self.generic_packet_check(&mut packet);
        if let Err((err1, err2)) = sender {
            // Err1 used if a packet has been sent while performing the checks (an error was found)
            if !err1.is_empty() {
                self.logger.log_error(err1.as_str());
            }
            self.logger.log_error(err2.as_str());
            return;
        }

        // TODO - Handle different packet types (fn call)
        match &mut packet.pack_type {
            PacketType::Ack(ack) => {}
            PacketType::Nack(nack) => {}
            PacketType::MsgFragment(fragment) => {}
            PacketType::FloodRequest(flood_request) => self.handle_flood_req(flood_request),
            PacketType::FloodResponse(_) => self.handle_flood_res(packet),
        }
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

/*FLOODING HANDLERS */
impl RustezeDrone {
    fn build_flood_response(flood_req: &FloodRequest) -> (NodeId, Packet) {
        let mut hops: Vec<NodeId> = flood_req.path_trace.iter().map(|(id, _)| *id).collect();
        hops.reverse();
        let dest = hops[1];

        let flood_res = FloodResponse {
            flood_id: flood_req.flood_id,
            path_trace: flood_req.path_trace.clone(),
        };
        let srh = SourceRoutingHeader { hop_index: 1, hops };
        let msg = Packet {
            pack_type: PacketType::FloodResponse(flood_res),
            routing_header: srh,
            session_id: 1,
        };

        (dest, msg)
    }

    fn send_flood_response(&self, dest: NodeId, flood_res: Packet) {
        match self.packet_senders.get(&dest) {
            Some(sender) => {
                sender.send(flood_res).unwrap();
            }
            None => {
                self.logger.log_error(
                    format!("[DRONE-{}][FLOOD RES] - Can't send to {}", self.id, dest).as_str(),
                );
            }
        }
    }

    fn handle_known_flood_id(&self, flood_req: &FloodRequest) {
        let (dest, msg) = Self::build_flood_response(flood_req);
        self.send_flood_response(dest, msg);
    }

    fn handle_new_flood_id(&self, flood_req: &FloodRequest) {
        // If drone has no neighbours except the sender of flood req
        if self.packet_senders.len() == 1 {
            let (dest, msg) = Self::build_flood_response(flood_req);
            self.send_flood_response(dest, msg);
            return;
        }

        // Forward flood req to neighbours
        for (id, sx) in &self.packet_senders {
            // Skip flood req sender
            let sender_id = flood_req.path_trace[flood_req.path_trace.len() - 2].0;
            if *id == sender_id {
                continue;
            }

            let msg = Packet {
                pack_type: PacketType::FloodRequest(flood_req.clone()),
                routing_header: SourceRoutingHeader {
                    hop_index: 0,
                    hops: vec![],
                },
                session_id: 1,
            };

            sx.send(msg).unwrap();
        }
    }

    fn handle_flood_req(&mut self, flood_req: &mut FloodRequest) {
        // Either case add the drone to the path trace
        flood_req.path_trace.push((self.id, NodeType::Drone));

        if !self.flood_history.insert(flood_req.flood_id) {
            self.handle_known_flood_id(flood_req);
            return;
        }

        self.handle_new_flood_id(flood_req);
    }

    /// Forward the flood response to the next hop
    fn handle_flood_res(&self, flood_res: Packet) {
        if let Some(dest) = flood_res.routing_header.get_current_hop() {
            self.send_flood_response(dest, flood_res);
            return;
        }

        self.logger
            .log_error(format!("[DRONE-{}][FLOOD RES] - No next hop", self.id).as_str());
    }
}
