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

impl RustezeDrone {
    /// Return the `NodeId` of the Drone
    pub fn get_id(&self) -> NodeId {
        self.id
    }

    fn print_log(&self, message: Result<(), String>, packet_str: String) {
        if let Err(err) = message {
            self.logger.log_error(err.as_str());
        } else {
            self.logger.log_info(
                format!(
                    "[DRONE-{}][{}] - {} handled successfully",
                    self.id, packet_str.to_ascii_uppercase(), packet_str
                )
                .as_str(),
            );
        }
    }

    fn get_packet_type(pt: &PacketType) -> String {
            match pt {
                PacketType::Ack(_) => "Ack".to_string(),
                PacketType::Nack(_) => "Nack".to_string(),
                PacketType::FloodRequest(_) => "Flood request".to_string(),
                PacketType::FloodResponse(_) => "Flood response".to_string(),
                PacketType::MsgFragment(_) => "Fragment".to_string(),
            }
    }

    fn check_next_hop(
        &mut self,
        current_node: NodeId,
        packet: &mut Packet,
    ) -> Result<Sender<Packet>, (String, String)> {
        let packet_str = Self::get_packet_type(&packet.pack_type);
        let mut send_res = "".to_string();
        // If current_node is wrong
        if current_node != self.id {
            if packet_str == "Fragment" {
                let res = self.build_send_nack(
                    packet.routing_header.hop_index,
                    packet.routing_header.clone(),
                    packet.session_id,
                    Nack {
                        fragment_index: packet.get_fragment_index(),
                        nack_type: NackType::UnexpectedRecipient(self.id),
                    },
                );
                if let Err(err) = res {
                    send_res = err;
                }
            }
            return Err((format!("[DRONE-{}][{}] - {} received by the wrong Node.\n Source routing header: {}", self.id, packet_str.to_ascii_uppercase(), packet_str, packet.routing_header), send_res));
        }

        // Increase hop index, since current_node is correct
        packet.routing_header.increase_hop_index();
        // Check if the new hop exists in neighbours
        match packet.routing_header.current_hop() {
            Some(next_node) => {
                let sender_res = get_sender(next_node, &self.packet_senders);
                match sender_res {
                    Ok(sender) => Ok(sender),
                    Err(err) => {
                        if packet_str == "Fragment" {
                            let res = self.build_send_nack(
                                packet.routing_header.hop_index-1,
                                packet.routing_header.clone(),
                                packet.session_id,
                                Nack {
                                    fragment_index: packet.get_fragment_index(),
                                    nack_type: NackType::ErrorInRouting(next_node),
                                },
                            );
                            if let Err(err) = res {
                                send_res = err;
                            }
                        }
                        Err((
                            format!("[DRONE-{}][{}] - {err}", self.id, packet_str),
                            send_res,
                        ))
                    }
                }
            }
            None => {
                if packet_str == "Fragment" {
                    let res = self.build_send_nack(
                        packet.routing_header.hop_index-1,
                        packet.routing_header.clone(),
                        packet.session_id,
                        Nack {
                            fragment_index: packet.get_fragment_index(),
                            nack_type: NackType::DestinationIsDrone,
                        },
                    );
                    if let Err(err) = res {
                        send_res = err;
                    }
                }
                Err((
                    format!("[DRONE-{}][ERR] - No next hop found", self.id),
                    send_res,
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
            let pt = Self::get_packet_type(&packet.pack_type);
            let mut send_res = (
                format!("[DRONE-{}][ERR] - No current hop found.\n Hops: {}\n", self.id, packet.routing_header),
                "".to_string(),
            );
            if pt == "Fragment" {
                let res = self.build_send_nack(
                    packet.routing_header.hop_index,
                    packet.routing_header.clone(),
                    packet.session_id,
                    Nack {
                        fragment_index: packet.get_fragment_index(),
                        nack_type: NackType::UnexpectedRecipient(self.id),
                    },
                );
                if let Err(err) = res {
                    send_res.1 = err;
                }
            }
            Err(send_res)
        }
    }

    fn packet_dispatcher(&mut self, mut packet: Packet) {
        let packet_str = Self::get_packet_type(&packet.pack_type);
        // If packet is a flood request skip checks
        let res;
        if let PacketType::FloodRequest(flood_req) = &mut packet.pack_type {
            res = self.handle_flood_req(flood_req);
            self.print_log(res, packet_str);
            return;
        }

        // Check if header is valid
        let sender = self.generic_packet_check(&mut packet);
        if let Err((err1, err2)) = sender {
            self.logger.log_error(err1.as_str());
            // Err2 used if a packet has been sent while performing the checks (an error was found)
            if !err2.is_empty() {
                self.logger.log_error(err1.as_str());
            }
            return;
        }

        let sender = sender.unwrap();

        res = match &mut packet.pack_type {
            PacketType::Ack(_) => self.send_ack(sender, packet),
            PacketType::Nack(_) => self.send_nack(sender, packet),
            PacketType::MsgFragment(_) => self.send_fragment(sender, packet),
            PacketType::FloodResponse(_) => self.handle_flood_res(sender, packet),
            _ => Err(format!(
                "[DRONE-{}][PACKET] - Unknown packet {}",
                self.id, packet
            )),
        };

        if let Err(err) = res {
            self.logger.log_error(err.as_str());
        } else {
            self.logger.log_info(
                format!(
                    "[DRONE-{}][{}] - Packet forwarded successfully",
                    self.id, packet_str
                )
                .as_str(),
            );
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
        let sender = hops[1];

        let packet = Packet::new_flood_response(
            SourceRoutingHeader { hop_index: 1, hops },
            1, // TODO - does it need to be retrieved from the flood request?
            FloodResponse {
                flood_id: flood_req.flood_id,
                path_trace: flood_req.path_trace.clone(),
            },
        );

        (sender, packet)
    }

    fn send_flood_response(&self, sender: NodeId, packet: Packet) -> Result<(), String> {
        let sender = get_sender(sender, &self.packet_senders);

        if let Err(err) = sender {
            return Err(format!(
                "[DRONE-{}][FLOOD RESPONSE] - Error occurred while sending flood response: {}",
                self.id, err
            ));
        }

        let sender = sender.unwrap();
        if let Err(err) = send_packet(&sender, packet) {
            return Err(format!(
                "[DRONE-{}][FLOOD RESPONSE] - Error occurred while sending flood response: {}",
                self.id, err
            ));
        }
        Ok(())
    }

    fn handle_known_flood_id(&self, flood_req: &FloodRequest) -> Result<(), String> {
        let (sender, msg) = Self::build_flood_response(flood_req);
        self.send_flood_response(sender, msg)
    }

    fn handle_new_flood_id(&self, flood_req: &FloodRequest) -> Result<(), String> {
        // If drone has no neighbours except the sender of flood req
        if self.packet_senders.len() == 1 {
            let (sender, msg) = Self::build_flood_response(flood_req);
            return self.send_flood_response(sender, msg);
        }
        let mut forward_res = String::new();
        // Forward flood req to neighbours
        for (id, sx) in &self.packet_senders {
            // Skip flood req sender
            let sender_id = flood_req.path_trace[flood_req.path_trace.len() - 2].0;
            if *id == sender_id {
                continue;
            }

            let packet = Packet::new_flood_request(
                SourceRoutingHeader {
                    hop_index: 0,
                    hops: vec![],
                },
                1,
                flood_req.clone(),
            );

            if let Err(err) = send_packet(sx, packet) {
                // Concat eventual errors while forwarding flood requests
                forward_res.push_str(&format!(
                    "[DRONE-{}][FLOOD REQUEST] - Error occurred while forwarding flood requests to DRONE {}. \n Error: {}\n",
                    self.id, id, err
                ));
            }
        }
        if !forward_res.is_empty() {
            return Err(forward_res);
        }
        Ok(())
    }

    fn handle_flood_req(&mut self, flood_req: &mut FloodRequest) -> Result<(), String> {
        // Either case add the drone to the path trace
        flood_req.path_trace.push((self.id, NodeType::Drone));

        if !self.flood_history.insert(flood_req.flood_id) {
            return self.handle_known_flood_id(flood_req);
        }

        self.handle_new_flood_id(flood_req)
    }

    /// Forward the flood response to the next hop
    fn handle_flood_res(&self, sender: Sender<Packet>, packet: Packet) -> Result<(), String> {
        send_packet(&sender, packet)
    }
}

/*ACK, NACK HANDLER */
impl RustezeDrone {
    fn send_ack(&self, sender: Sender<Packet>, packet: Packet) -> Result<(), String> {
        if let Err(err) = send_packet(&sender, packet) {
            return Err(format!(
                "[DRONE-{}][ACK] - Error occurred while sending ack: {}",
                self.id, err
            ));
        }
        Ok(())
    }

    fn send_nack(&self, sender: Sender<Packet>, packet: Packet) -> Result<(), String> {
        if let Err(err) = send_packet(&sender, packet) {
            return Err(format!(
                "[DRONE-{}][NACK] - Error occurred while sending nack: {}",
                self.id, err
            ));
        }
        Ok(())
    }

    fn build_send_nack(
        &self,
        index : usize,
        routing_header: SourceRoutingHeader,
        session_id: u64,
        nack: Nack,
    ) -> Result<(), String> {
        // Build the Nack and reverse the packet's the route.
        let source_routing_header = routing_header.sub_route(..index);
        if source_routing_header.is_none() {
            return Err(format!(
                    "[DRONE-{}][FRAGMENT] - Unable to retrieve source routing header sub-route. \n Hops: {} \n Hop index: {}",
                    self.id, routing_header, routing_header.hop_index
                ));
        }
        let mut source_routing_header = source_routing_header.unwrap();
        source_routing_header.reverse();

        let packet = Packet::new_nack(source_routing_header, session_id, nack);

        // Retrieve the senderination (Sender) to send the Nack.
        let sender = get_sender(
            routing_header.current_hop().unwrap_or(0),
            &self.packet_senders,
        );
        if let Err(err) = sender {
            return Err(format!(
                "[DRONE-{}][NACK] - Error occurred while sending nack: {}",
                self.id, err
            ));
        }

        self.send_ack(sender.unwrap(), packet)
    }
}

/*FRAGMENT HANDLER */
impl RustezeDrone {
    fn to_drop(&self) -> bool {
        let mut rng = rand::thread_rng();
        let random_value: f32 = rng.gen();
        self.pdr > random_value
    }

    fn send_fragment(&self, sender: Sender<Packet>, packet: Packet) -> Result<(), String> {
        if self.to_drop() {
            let source_routing_header = packet
                .routing_header
                .sub_route(..packet.routing_header.hop_index);
            if source_routing_header.is_none() {
                return Err(format!(
                    "[DRONE-{}][FRAGMENT] - Unable to retrieve source routing header sub-route. \n Hops: {} \n Hop index: {}",
                    self.id, packet.routing_header, packet.routing_header.hop_index
                ));
            }
            let mut source_routing_header = source_routing_header.unwrap();
            source_routing_header.reverse();

            let packet = Packet::new_nack(
                source_routing_header,
                packet.session_id,
                Nack {
                    fragment_index: packet.get_fragment_index(),
                    nack_type: NackType::Dropped,
                },
            );
            let res = send_packet(&sender, packet);
            if let Err(err) = res {
                return Err(format!(
                    "[DRONE-{}][FRAGMENT] - Error occurred while sending NACK for dropped fragment: {}",
                    self.id, err
                ));
            }
        } else {
            let res = send_packet(&sender, packet);
            if let Err(err) = res {
                return Err(format!(
                    "[DRONE-{}][FRAGMENT] - Error occurred while sending fragment: {}",
                    self.id, err
                ));
            }
        }
        Ok(())
    }
}
