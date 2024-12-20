use super::RustezeDrone;

use crossbeam::channel::Sender;
use wg_internal::network::NodeId;
use wg_internal::packet::{Nack, NackType, Packet, PacketType};

use crate::packet_send::{get_sender, send_packet};

/* MAIN PACKETS HANDLER */
impl RustezeDrone {
    #[must_use]
    /// Return the `NodeId` of the Drone
    pub fn get_id(&self) -> NodeId {
        self.id
    }

    pub(crate) fn print_log(&self, message: &Result<(), String>, packet_str: &String) {
        if let Err(err) = message {
            if err.contains("dropped") {
                self.logger.log_warn(err.as_str());
            } else {
                self.logger.log_error(err.as_str());
            }
        } else {
            self.logger.log_debug(
                format!(
                    "[DRONE-{}][{}] - {} handled successfully",
                    self.id,
                    packet_str.to_ascii_uppercase(),
                    packet_str
                )
                .as_str(),
            );
        }
    }

    pub(crate) fn get_packet_type(pt: &PacketType) -> String {
        match pt {
            PacketType::Ack(_) => "Ack".to_string(),
            PacketType::Nack(_) => "Nack".to_string(),
            PacketType::FloodRequest(_) => "Flood request".to_string(),
            PacketType::FloodResponse(_) => "Flood response".to_string(),
            PacketType::MsgFragment(_) => "Fragment".to_string(),
        }
    }

    pub(crate) fn check_next_hop(
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
                    &packet.routing_header,
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
        if let Some(next_node) = packet.routing_header.current_hop() {
            let sender_res = get_sender(next_node, &self.packet_senders);
            match sender_res {
                Ok(sender) => Ok(sender),
                Err(err) => {
                    if let PacketType::MsgFragment(_) = &packet.pack_type {
                        let res = self.build_send_nack(
                            packet.routing_header.hop_index,
                            &packet.routing_header,
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
        } else {
            if let PacketType::MsgFragment(_) = &packet.pack_type {
                let res = self.build_send_nack(
                    packet.routing_header.hop_index,
                    &packet.routing_header,
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

    pub(crate) fn generic_packet_check(
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
                &packet.routing_header,
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

    pub(crate) fn packet_dispatcher(&mut self, mut packet: Packet) {
        let packet_str = Self::get_packet_type(&packet.pack_type);
        // If packet is a flood request skip checks
        let res;
        if self.terminated {
            self.logger.log_warn(
                format!(
                    "[DRONE-{}][FLOOD REQUEST] - Drone is terminated. Ignoring packet: {}",
                    self.id, packet
                )
                .as_str(),
            );
            return;
        }
            if let PacketType::FloodRequest(flood_req) = &mut packet.pack_type {
                res = self.handle_flood_req(flood_req);
                self.print_log(&res, &packet_str);
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

        let mut forward_packet = packet.clone();
        res = match &mut packet.pack_type {
            PacketType::Ack(_) => self.send_ack(&sender, &forward_packet),
            PacketType::Nack(_) => self.send_nack(&sender, &forward_packet),
            PacketType::MsgFragment(_) => {
                if self.terminated {
                    self.build_send_nack(
                        packet.routing_header.hop_index,
                        &packet.routing_header,
                        packet.session_id,
                        Nack {
                            fragment_index: packet.get_fragment_index(),
                            nack_type: NackType::ErrorInRouting(self.id),
                        },
                    )
                } else {
                    self.send_fragment(&sender, &mut forward_packet)
                }
            }
            PacketType::FloodResponse(_) => send_packet(&sender, &packet),
            PacketType::FloodRequest(_) => Err(format!(
                "[DRONE-{}][PACKET] - Unknown packet {}",
                self.id, packet
            )),
        };

        // Print packet forwarding result
        self.print_log(&res, &packet_str);

        // If packet is sent successfully, send event to SC
        if let Ok(()) = res {
            self.event_dispatcher(&packet, &packet_str);
        }
    }
}
