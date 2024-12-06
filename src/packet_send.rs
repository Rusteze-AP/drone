use crossbeam::channel::Sender;
use std::collections::HashMap;
use wg_internal::network::{NodeId, SourceRoutingHeader};
use wg_internal::packet::{FloodRequest, FloodResponse, Fragment, Nack, NackType, Packet};

pub fn get_sender(
    node_id: NodeId,
    senders: &HashMap<NodeId, Sender<Packet>>,
) -> Result<Sender<Packet>, String> {
    if let Some(sender) = senders.get(&node_id) {
        return Ok(sender.clone());
    }
    Err(format!("No neigbour of ID [{node_id}] found."))
}

pub fn send_fragment(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    fragment: Fragment,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_fragment(routing_header, session_id, fragment);
    match sender.send(packet) {
        Ok(()) => Ok(()),
        Err(err) => Err(format!(
            "[DRONE-{drone_id}][FRAGMENT] - Error sending fragment: {err}"
        )),
    }
}

pub fn send_ack(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    fragment_index: u64,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_ack(routing_header, session_id, fragment_index);

    match sender.send(packet) {
        Ok(()) => Ok(()),
        Err(err) => Err(format!(
            "[DRONE-{}][ACK] - Error sending ack: {}",
            drone_id, err
        )),
    }
}

pub fn send_nack(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    fragment_index: u64,
    nack_type: NackType,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_nack(
        routing_header,
        session_id,
        Nack {
            fragment_index,
            nack_type,
        },
    );

    send_packet(sender, packet)
}

pub fn send_flood_request(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    flood_request: FloodRequest,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_flood_request(routing_header, session_id, flood_request);

    send_packet(sender, packet)
}

pub fn send_flood_response(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    flood_response: FloodResponse,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_flood_response(routing_header, session_id, flood_response);

    send_packet(sender, packet)
}

pub fn send_packet(sender: &Sender<Packet>, packet: Packet) -> Result<(), String> {
    match sender.send(packet.clone()) {
        Ok(()) => Ok(()),
        Err(err) => Err(format!(
            "Tried sending packet: {packet} but an error occurred: {err}"
        )),
    }
}
