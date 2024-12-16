mod flood_handler;
mod fragment_handler;
mod logger_setting;
mod packet_handler;
mod response_handler;
mod sc_handler;

use crossbeam::channel::{select_biased, Receiver, Sender};
use logger::{LogLevel, Logger};
use std::collections::{HashMap, HashSet};
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::drone::Drone;
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

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

impl RustezeDrone {
    fn internal_run(&mut self) {
        loop {
            if self.terminated {
                if let Ok(msg) = self.packet_recv.recv() {
                    self.packet_dispatcher(msg);
                } else {
                    self.logger.log_error(format!("[DRONE-{}][RUNNER] - Drone receiver disconnected. Terminating thread...", self.id).as_str());
                    break;
                }
            } else {
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
            logger: Logger::new(LogLevel::None as u8, false, "RustezeDrone".to_string()),
        }
    }

    fn run(&mut self) {
        self.internal_run();
    }
}
