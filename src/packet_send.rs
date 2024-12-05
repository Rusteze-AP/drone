use crossbeam::channel::Sender;
use std::collections::HashMap;
use wg_internal::network::{NodeId, SourceRoutingHeader};
use wg_internal::packet::{FloodRequest, FloodResponse, Fragment, Nack, NackType, Packet};

fn get_sender(
    node_id: Option<NodeId>,
    senders: &HashMap<NodeId, Sender<Packet>>,
) -> Result<Sender<Packet>, String> {
    if let Some(id) = node_id {
        if let Some(sender) = senders.get(&id) {
            return Ok(sender.clone());
        }
    }
    Err("todo".to_string())
}

fn send_fragment(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    fragment: Fragment,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_fragment(routing_header, session_id, fragment);
    match sender.send(packet) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!(
            "[DRONE-{}][FRAGMENT] - Error sending fragment: {}",
            drone_id, err
        )),
    }
}

fn send_ack(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    fragment_index: u64,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_ack(routing_header, session_id, fragment_index);

    match sender.send(packet) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!(
            "[DRONE-{}][ACK] - Error sending ack: {}",
            drone_id, err
        )),
    }
}

fn send_nack(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    fragment_index: u64,
    nack_type: NackType,
    sender: Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_nack(
        routing_header,
        session_id,
        Nack {
            fragment_index,
            nack_type,
        },
    );

    match sender.send(packet) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!(
            "[DRONE-{}][NACK] - Error sending nack: {}",
            drone_id, err
        )),
    }
}

fn send_flood_request(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    flood_request: FloodRequest,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_flood_request(routing_header, session_id, flood_request);

    match sender.send(packet) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!(
            "[DRONE-{}][FLOOD REQUEST] - Error sending flood request: {}",
            drone_id, err
        )),
    }
}

fn send_flood_response(
    drone_id: NodeId,
    routing_header: SourceRoutingHeader,
    session_id: u64,
    flood_response: FloodResponse,
    sender: &Sender<Packet>,
) -> Result<(), String> {
    let packet = Packet::new_flood_response(routing_header, session_id, flood_response);

    match sender.send(packet) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!(
            "[DRONE-{}][FLOOD RESPONSE] - Error sending flood response: {}",
            drone_id, err
        )),
    }
}
