// use crate::messages::{RustezePacket, RustezeSourceRoutingHeader};
use crossbeam::channel::{select_biased, Receiver, Sender};
use logger::Logger;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::drone::Drone;
use wg_internal::network::{NodeId, SourceRoutingHeader};
use wg_internal::packet::{
    FloodRequest, FloodResponse, Nack, NackType, NodeType, Packet, PacketType,
};

use crate::packet_send::{get_sender, sc_send_packet, send_packet};

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
    #[must_use]
    pub fn get_id(&self) -> NodeId {
        self.id
    }

    fn print_log(&self, message: Result<(), String>, packet_str: String) {
        if let Err(err) = message {
            if err == "DROPPED" {
                self.logger.log_debug(
                    format!(
                        "[DRONE-{}][{}] - {} dropped",
                        self.id,
                        packet_str.to_ascii_uppercase(),
                        packet_str
                    )
                    .as_str(),
                );
            } else {
                self.logger.log_error(err.as_str());
            }
        } else {
            self.logger.log_debug(
                format!(
                    "[DRONE-{}][{}] - {} forwarded successfully",
                    self.id,
                    packet_str.to_ascii_uppercase(),
                    packet_str
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
        let mut send_res = String::new();
        // If current_node is wrong
        if current_node != self.id {
            if let PacketType::MsgFragment(_) = &packet.pack_type {
                let res = self.build_send_nack(
                    packet.routing_header.hop_index + 1,
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
            return Err((
                format!(
                    "[DRONE-{}][{}] - {} received by the wrong Node.\n Source routing header: {}",
                    self.id,
                    packet_str.to_ascii_uppercase(),
                    packet_str,
                    packet.routing_header
                ),
                send_res,
            ));
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
                        if let PacketType::MsgFragment(_) = &packet.pack_type {
                            let res = self.build_send_nack(
                                packet.routing_header.hop_index,
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
                if let PacketType::MsgFragment(_) = &packet.pack_type {
                    let res = self.build_send_nack(
                        packet.routing_header.hop_index,
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
        }

        let mut send_res = (
            format!(
                "[DRONE-{}][ERR] - No current hop found.\n Hops: {}\n",
                self.id, packet.routing_header
            ),
            String::new(),
        );
        if let PacketType::MsgFragment(_) = &packet.pack_type {
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

    fn packet_dispatcher(&mut self, mut packet: Packet) {
        let packet_str = Self::get_packet_type(&packet.pack_type);
        // If packet is a flood request skip checks
        let res;
        if !self.terminated {
            if let PacketType::FloodRequest(flood_req) = &mut packet.pack_type {
                res = self.handle_flood_req(flood_req);
                self.print_log(res, packet_str);
                return;
            }
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
            PacketType::MsgFragment(_) => {
                if !self.terminated {
                    self.send_fragment(sender, packet)
                } else {
                    self.build_send_nack(
                        packet.routing_header.hop_index,
                        packet.routing_header.clone(),
                        packet.session_id,
                        Nack {
                            fragment_index: packet.get_fragment_index(),
                            nack_type: NackType::ErrorInRouting(self.id),
                        },
                    )
                }
            }
            PacketType::FloodResponse(_) => self.handle_flood_res(sender, packet),
            PacketType::FloodRequest(_) => {
                if !self.terminated {
                    Err(format!(
                        "[DRONE-{}][PACKET] - Unknown packet {}",
                        self.id, packet
                    ))
                } else {
                    Ok(())
                }
            }
        };

        self.print_log(res, packet_str);
    }

    fn internal_run(&mut self) {
        loop {
            if self.terminated {
                self.logger
                    .log_debug(format!("[DRONE-{}][RUNNER] - Terminated", self.id).as_str());
                break;
            }
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.command_dispatcher(command);
                    } else {
                        self.logger.log_error(format!("[DRONE-{}][RUNNER] - Simulation controller receiver disconnected. Terminating thread...", self.id).as_str());
                        break;
                    }
                }
                recv(self.packet_recv) -> msg => {
                    if let Ok(msg) = msg {
                        self.packet_dispatcher(msg);
                    } else {
                        self.logger.log_error(format!("[DRONE-{}][RUNNER] - Drone receiver disconnected. Terminating thread...", self.id).as_str());
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
        if let Err(err) = send_packet(&sender, &packet) {
            // Send to SC
            let res = sc_send_packet(
                &self.controller_send,
                &DroneEvent::ControllerShortcut(packet),
            );
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

            if let Err(err) = send_packet(sx, &packet) {
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
        send_packet(&sender, &packet)
    }
}

/* DRONE EVENT */

impl RustezeDrone {}

/*ACK, NACK HANDLER */
impl RustezeDrone {
    fn send_ack(&self, sender: Sender<Packet>, packet: Packet) -> Result<(), String> {
        if let Err(err) = send_packet(&sender, &packet) {
            // Send to SC
            let res = sc_send_packet(
                &self.controller_send,
                &DroneEvent::ControllerShortcut(packet),
            );
            return Err(format!(
                "[DRONE-{}][ACK] - Error occurred while sending ack: {}",
                self.id, err
            ));
        }
        let res = sc_send_packet(&self.controller_send, &DroneEvent::PacketSent(packet));
        Ok(())
    }

    fn send_nack(&self, sender: Sender<Packet>, packet: Packet) -> Result<(), String> {
        if let Err(err) = send_packet(&sender, &packet) {
            // Send to SC
            let res = sc_send_packet(
                &self.controller_send,
                &DroneEvent::ControllerShortcut(packet),
            );
            return Err(format!(
                "[DRONE-{}][NACK] - Error occurred while sending nack: {}",
                self.id, err
            ));
        }
        let res = sc_send_packet(&self.controller_send, &DroneEvent::PacketSent(packet));
        Ok(())
    }

    /// This function builds a Nack packet that needs to be forwarded back to its source.
    /// It will reverse the packet route and forward it.
    ///
    /// # Arguments
    /// * `index` - The index at which to split the route (excluded).
    /// * `routing_header` - The current routing header of the packet.
    /// * `session_id` - The session id of the packet.
    /// * `nack` - The Nack packet to be sent.
    fn build_send_nack(
        &self,
        index: usize,
        routing_header: SourceRoutingHeader,
        session_id: u64,
        nack: Nack,
    ) -> Result<(), String> {
        // Build the Nack and reverse the packet's the route.
        let source_routing_header = routing_header.sub_route(..index);
        if source_routing_header.is_none() {
            return Err(format!(
                    "[DRONE-{}][NACK] - Unable to retrieve source routing header sub-route. \n Hops: {} \n Hop index: {}",
                    self.id, routing_header, routing_header.hop_index
                ));
        }

        let mut new_routing_header = source_routing_header.unwrap();
        new_routing_header.hops.reverse(); // Reverse in place
        new_routing_header.hop_index = 1; // Set hop_index to 1 (next hop)

        let packet = Packet::new_nack(new_routing_header.clone(), session_id, nack);

        // Retrieve the sender to send the Nack.
        let sender = get_sender(
            new_routing_header.current_hop().unwrap_or(0),
            &self.packet_senders,
        );
        if let Err(err) = sender {
            return Err(format!("{}", err));
        }
        // let sender = sender.unwrap();
        if let Err(err) = send_packet(&sender.unwrap(), &packet) {
            // Send to SC
            let res = sc_send_packet(
                &self.controller_send,
                &DroneEvent::ControllerShortcut(packet),
            );
            return Err(format!("Error occurred while sending nack: {}", err));
        }
        let res = sc_send_packet(&self.controller_send, &DroneEvent::PacketSent(packet));
        Ok(())
    }
}

/*FRAGMENT HANDLER */
impl RustezeDrone {
    fn to_drop(&self) -> bool {
        let mut rng = rand::thread_rng();
        let random_value: f32 = rng.gen();
        self.pdr > random_value
    }

    fn send_fragment(&self, sender: Sender<Packet>, mut packet: Packet) -> Result<(), String> {
        if self.to_drop() {
            packet.routing_header.decrease_hop_index(); // Hop index has been increased before to check the next hop
            let res = sc_send_packet(&self.controller_send, &DroneEvent::PacketDropped(packet.clone()));
            let res = self.build_send_nack(
                packet.routing_header.hop_index+1,
                packet.routing_header.clone(),
                packet.session_id,
                Nack {
                    fragment_index: packet.get_fragment_index(),
                    nack_type: NackType::Dropped,
                },
            );
            if let Err(err) = res {
                return Err(format!(
                    "[DRONE-{}][FRAGMENT] - Error occurred while sending NACK for \"to drop\" fragment: {}",
                    self.id, err
                ));
            }
            return Err("DROPPED".to_string());
        } else {
            let res = send_packet(&sender, &packet);
            if let Err(err) = res {
                return Err(format!(
                    "[DRONE-{}][FRAGMENT] - Error occurred while sending fragment: {}",
                    self.id, err
                ));
            }
        }
        let res = sc_send_packet(&self.controller_send, &DroneEvent::PacketSent(packet));
        Ok(())
    }
}

/*COMMANDS HANDLER */
impl RustezeDrone {
    fn set_pdr(&mut self, new_pdr: f32) {
        self.pdr = new_pdr;
    }

    fn remove_sender(&mut self, node_id: NodeId) -> Result<(), String> {
        let res = self.packet_senders.remove(&node_id);
        if res.is_none() {
            Err(format!(
                "[DRONE-{}][REMOVE SENDER] - Sender with id {} not found",
                self.id, node_id
            ))
        } else {
            Ok(())
        }
    }

    fn add_sender(&mut self, id: NodeId, sender: Sender<Packet>) -> Result<(), String> {
        let res = self.packet_senders.insert(id, sender);
        if res.is_some() {
            Err(format!(
                "[DRONE-{}][ADD SENDER] - Sender with id {} already exists",
                self.id, id
            ))
        } else {
            Ok(())
        }
    }

    fn crash(&mut self) -> Result<(), String> {
        //TODO handle drone crash
        self.terminated = true;
        Ok(())
    }

    fn command_dispatcher(&mut self, command: DroneCommand) {
        if !self.terminated {
            let res = match command {
                DroneCommand::RemoveSender(node_id) => self.remove_sender(node_id),
                DroneCommand::AddSender(id, sender) => self.add_sender(id, sender),
                DroneCommand::SetPacketDropRate(new_pdr) => {
                    self.set_pdr(new_pdr);
                    Ok(())
                }
                DroneCommand::Crash => self.crash(),
            };

            if let Err(err) = res {
                self.logger.log_error(err.as_str());
            }
        }
    }
}
