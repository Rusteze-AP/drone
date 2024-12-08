use crossbeam::channel::Sender;
use std::collections::HashMap;
use wg_internal::controller::DroneEvent;
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

pub fn get_sender(
    node_id: NodeId,
    senders: &HashMap<NodeId, Sender<Packet>>,
) -> Result<Sender<Packet>, String> {
    if let Some(sender) = senders.get(&node_id) {
        return Ok(sender.clone());
    }
    Err(format!("No neigbour of ID [{node_id}] found."))
}

pub fn send_packet(sender: &Sender<Packet>, packet: &Packet) -> Result<(), String> {
    match sender.send(packet.clone()) {
        Ok(()) => Ok(()),
        Err(err) => Err(format!(
            "Tried sending packet: {packet} but an error occurred: {err}"
        )),
    }
}

pub fn sc_send_packet(sender: &Sender<DroneEvent>, packet: &DroneEvent) -> Result<(), String> {
    match sender.send(packet.clone()) {
        Ok(()) => Ok(()),
        Err(err) => Err(format!(
            "Error occurred while sending packet event to SC. Error: {}",
            err
        )),
    }
}
