use super::RustezeDrone;

use crossbeam::channel::Sender;
use wg_internal::controller::DroneEvent;
use wg_internal::network::SourceRoutingHeader;
use wg_internal::packet::{Nack, Packet};

use crate::packet_send::{get_sender, sc_send_packet, send_packet};

/*ACK, NACK HANDLER */
impl RustezeDrone {
    pub(crate) fn send_ack(&self, sender: &Sender<Packet>, packet: &Packet) -> Result<(), String> {
        if let Err(err) = send_packet(sender, packet) {
            self.logger.log_warn(format!("[DRONE-{}][ACK] - Failed to forward packet to [DRONE-{}]. \n Error: {} \n Trying to use SC shortcut...", self.id, packet.routing_header.current_hop().unwrap(), err).as_str());
            // Send to SC
            let res = sc_send_packet(
                &self.controller_send,
                &DroneEvent::ControllerShortcut(packet.clone()),
            );

            if let Err(err) = res {
                self.logger
                    .log_error(format!("[DRONE-{}][ACK] - {}", self.id, err).as_str());
                return Err(format!(
                    "[DRONE-{}][ACK] - Unable to forward packet to neither next hop nor SC. \n Packet: {}",
                    self.id, packet
                ));
            }
            self.logger.log_debug(
                format!(
                    "[DRONE-{}][ACK] - Successfully sent ACK through SC. Packet: {}",
                    self.id, packet
                )
                .as_str(),
            );
        }
        Ok(())
    }

    pub(crate) fn send_nack(&self, sender: &Sender<Packet>, packet: &Packet) -> Result<(), String> {
        if let Err(err) = send_packet(sender, packet) {
            self.logger.log_warn(format!("[DRONE-{}][NACK] - Failed to forward packet to [DRONE-{}]. \n Error: {} \n Trying to use SC shortcut...", self.id, packet.routing_header.current_hop().unwrap(), err).as_str());
            // Send to SC
            let res = sc_send_packet(
                &self.controller_send,
                &DroneEvent::ControllerShortcut(packet.clone()),
            );

            if let Err(err) = res {
                self.logger
                    .log_error(format!("[DRONE-{}][NACK] - {}", self.id, err).as_str());
                return Err(format!(
                    "[DRONE-{}][NACK] - Unable to forward packet to neither next hop nor SC. \n Packet: {}",
                    self.id, packet
                ));
            }
            self.logger.log_debug(
                format!(
                    "[DRONE-{}][NACK] - Successfully sent NACK through SC. Packet: {}",
                    self.id, packet
                )
                .as_str(),
            );
        }
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
    pub(crate) fn build_send_nack(
        &self,
        index: usize,
        routing_header: &SourceRoutingHeader,
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
            return Err(format!(
                "[DRONE-{}][NACK] - Error occurred while sending NACK: {}",
                self.id, err
            ));
        }

        self.send_nack(&sender.unwrap(), &packet)
    }
}
