use super::RustezeDrone;

use crossbeam::channel::Sender;
use rand::Rng;
use wg_internal::controller::DroneEvent;
use wg_internal::packet::{Nack, NackType, Packet};

use crate::packet_send::{sc_send_packet, send_packet};

/*FRAGMENT HANDLER */
impl RustezeDrone {
    pub(crate) fn to_drop(&self) -> bool {
        let mut rng = rand::thread_rng();
        let random_value: f32 = rng.gen();
        self.pdr > random_value
    }

    pub(crate) fn send_fragment(
        &self,
        sender: &Sender<Packet>,
        mut packet: Packet,
    ) -> Result<(), String> {
        if self.to_drop() {
            packet.routing_header.decrease_hop_index(); // Hop index has been increased before to check the next hop
            let res = sc_send_packet(
                &self.controller_send,
                &DroneEvent::PacketDropped(packet.clone()),
            );
            if let Err(err) = res {
                self.logger
                    .log_error(format!("[DRONE-{}][FRAGMENT] - {}", self.id, err).as_str());
            }
            let res = self.build_send_nack(
                packet.routing_header.hop_index + 1,
                &packet.routing_header,
                packet.session_id,
                Nack {
                    fragment_index: packet.get_fragment_index(),
                    nack_type: NackType::Dropped,
                },
            );
            if let Err(err) = res {
                return Err(format!(
                    "[DRONE-{}][FRAGMENT] - Error occurred while sending NACK for \"to drop\" fragment. \n Error: {}",
                    self.id, err
                ));
            }
            return Err(format!("[DRONE-{}][FRAGMENT] - Fragment dropped", self.id,));
        }

        let res = send_packet(sender, &packet);
        if let Err(err) = res {
            return Err(format!(
                "[DRONE-{}][FRAGMENT] - Error occurred while sending fragment: {}",
                self.id, err
            ));
        }
        Ok(())
    }
}
