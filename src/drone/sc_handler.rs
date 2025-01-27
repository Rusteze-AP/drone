use super::RustezeDrone;

use crossbeam::channel::Sender;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

use crate::packet_send::sc_send_packet;

/*COMMANDS & EVENT HANDLERs */
impl RustezeDrone {
    pub(crate) fn set_pdr(&mut self, new_pdr: f32) {
        self.pdr = new_pdr;
        self.logger.log_debug(
            format!(
                "[DRONE-{}][SET PDR] - Packet drop rate set to {}",
                self.id, self.pdr
            )
            .as_str(),
        );
    }

    pub(crate) fn remove_sender(&mut self, node_id: NodeId) -> Result<(), String> {
        let res = self.packet_senders.remove(&node_id);
        if res.is_none() {
            Err(format!(
                "[DRONE-{}][REMOVE SENDER] - Sender with id {} not found",
                self.id, node_id
            ))
        } else {
            self.logger.log_debug(
                format!(
                    "[DRONE-{}][REMOVE SENDER] - Sender with id {} removed",
                    self.id, node_id
                )
                .as_str(),
            );
            Ok(())
        }
    }

    pub(crate) fn add_sender(&mut self, id: NodeId, sender: &Sender<Packet>) -> Result<(), String> {
        let res = self.packet_senders.insert(id, sender.clone());
        if res.is_some() {
            Err(format!(
                "[DRONE-{}][ADD SENDER] - Sender with id {} already exists",
                self.id, id
            ))
        } else {
            self.logger.log_debug(
                format!(
                    "[DRONE-{}][ADD SENDER] - Sender with id {} added",
                    self.id, id
                )
                .as_str(),
            );
            Ok(())
        }
    }

    pub(crate) fn crash(&mut self) -> Result<(), String> {
        self.logger.log_debug(
            format!(
                "[DRONE-{}][CRASH] Drone entered crash sequence. Terminating... - ",
                self.id
            )
            .as_str(),
        );
        self.terminated = true;
        Ok(())
    }

    pub(crate) fn command_dispatcher(&mut self, command: DroneCommand) {
        if !self.terminated {
            let res = match command {
                DroneCommand::RemoveSender(node_id) => self.remove_sender(node_id),
                DroneCommand::AddSender(id, sender) => self.add_sender(id, &sender),
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

    pub(crate) fn event_dispatcher(&self, packet: &Packet, packet_str: &str) {
        let res = sc_send_packet(
            &self.controller_send,
            &DroneEvent::PacketSent(packet.clone()),
        );
        if let Err(err) = res {
            self.logger.log_error(
                format!(
                    "[DRONE-{}][{}] - Packet event forward: {}",
                    self.id,
                    packet_str.to_ascii_uppercase(),
                    err
                )
                .as_str(),
            );
            return;
        }
        self.logger.log_debug(
            format!(
                "[DRONE-{}][{}] - Packet event sent successfully",
                self.id,
                packet_str.to_ascii_uppercase()
            )
            .as_str(),
        );
    }
}
