use super::RustezeDrone;

use wg_internal::controller::DroneEvent;
use wg_internal::network::{NodeId, SourceRoutingHeader};
use wg_internal::packet::{FloodRequest, NodeType, Packet};

use crate::packet_send::{get_sender, sc_send_packet, send_packet};

/*FLOODING HANDLERS */
impl RustezeDrone {
    pub(crate) fn build_flood_response(flood_req: &FloodRequest) -> (NodeId, Packet) {
        let mut packet = flood_req.generate_response(1); // Note: returns with hop_index = 0;
        packet.routing_header.increase_hop_index();
        let dest = packet.routing_header.current_hop();

        if dest.is_none() {
            return (0, packet);
        }

        (dest.unwrap(), packet)
    }

    pub(crate) fn send_flood_response(
        &self,
        sender: NodeId,
        packet: &Packet,
    ) -> Result<(), String> {
        let sender = get_sender(sender, &self.packet_senders);

        if let Err(err) = sender {
            return Err(format!(
                "[DRONE-{}][FLOOD RESPONSE] - Error occurred while sending flood response: {}",
                self.id, err
            ));
        }

        let sender = sender.unwrap();
        if let Err(err) = send_packet(&sender, packet) {
            self.logger.log_warn(format!("[DRONE-{}][FLOOD RESPONSE] - Failed to forward packet to [DRONE-{}]. \n Error: {} \n Trying to use SC shortcut...", self.id, packet.routing_header.current_hop().unwrap(), err).as_str());
            // Send to SC
            let res = sc_send_packet(
                &self.controller_send,
                &DroneEvent::ControllerShortcut(packet.clone()),
            );

            if let Err(err) = res {
                self.logger
                    .log_error(format!("[DRONE-{}][FLOOD RESPONSE] - {}", self.id, err).as_str());
                return Err(format!(
                    "[DRONE-{}][FLOOD RESPONSE] - Unable to forward packet to neither next hop nor SC. \n Packet: {}",
                    self.id, packet
                ));
            }

            self.logger.log_debug(
                format!(
                    "[DRONE-{}][FLOOD RESPONSE] - Successfully sent flood response through SC. Packet: {}",
                    self.id, packet
                )
                .as_str(),
            );
        }
        Ok(())
    }

    pub(crate) fn handle_known_flood_id(&self, flood_req: &FloodRequest) -> Result<(), String> {
        let (sender, msg) = Self::build_flood_response(flood_req);
        self.send_flood_response(sender, &msg)
    }

    pub(crate) fn handle_new_flood_id(&self, flood_req: &FloodRequest) -> Result<(), String> {
        // If drone has no neighbours except the sender of flood req
        if self.packet_senders.len() == 1 {
            let (sender, msg) = Self::build_flood_response(flood_req);
            return self.send_flood_response(sender, &msg);
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

    pub(crate) fn handle_flood_req(&mut self, flood_req: &mut FloodRequest) -> Result<(), String> {
        // Either case add the drone to the path trace
        flood_req.path_trace.push((self.id, NodeType::Drone));

        if !self
            .flood_history
            .insert((flood_req.initiator_id, flood_req.flood_id))
        {
            return self.handle_known_flood_id(flood_req);
        }

        self.handle_new_flood_id(flood_req)
    }
}
