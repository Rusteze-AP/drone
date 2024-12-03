use crossbeam::channel::unbounded;
use std::collections::HashMap;
use std::thread;
use wg_internal::drone::Drone;
use wg_internal::network::SourceRoutingHeader;
use wg_internal::packet::{FloodRequest, FloodResponse, NodeType};
use wg_internal::packet::{Packet, PacketType};

fn create_sample_flood_req(flood_id: u64) -> Packet {
    Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id,
            initiator_id: 1,
            path_trace: vec![(1, NodeType::Client)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: Vec::new(),
        },
        session_id: 1,
    }
}

/// This function checks whether a drone builds a flood response packet correctly.
pub fn generic_new_flood<T: Drone + Send + 'static>() {
    // Client 1
    let (c_send, c_recv) = unbounded::<Packet>();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    let (_d_command_send, d_command_recv) = unbounded();

    let mut drone = T::new(
        11,
        unbounded().0,
        d_command_recv,
        d_recv.clone(),
        HashMap::from([(1, c_send.clone())]),
        0.0,
    );

    thread::spawn(move || {
        drone.run();
    });

    let msg = create_sample_flood_req(1);
    // Client sends packet to d
    d_send.send(msg.clone()).unwrap();

    let flood_res = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse {
            flood_id: 1,
            path_trace: vec![(1, NodeType::Client), (11, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![11, 1],
        },
        session_id: 1,
    };

    // Client receive a flood response originated from 'd'
    assert_eq!(c_recv.recv().unwrap(), flood_res);
}

/// This function checks if a flood request is forwarded to all neighbours of a drone (excluding the sender) and waits for 2 responses.
pub fn generic_new_flood_neighbours<T: Drone + Send + 'static>() {
    let (c_send, c_recv) = unbounded::<Packet>();
    let (d_send, d_recv) = unbounded();
    let (d2_send, d2_recv) = unbounded::<Packet>();
    let (d3_send, d3_recv) = unbounded::<Packet>();
    let (_d_command_send, d_command_recv) = unbounded();

    let neighbours = HashMap::from([
        (1, c_send.clone()),
        (12, d2_send.clone()),
        (13, d3_send.clone()),
    ]);
    let mut drone = T::new(
        11,
        unbounded().0,
        d_command_recv.clone(),
        d_recv.clone(),
        neighbours,
        0.0,
    );

    thread::spawn(move || {
        drone.run();
    });

    let neighbours = HashMap::from([(11, d_send.clone())]);
    let mut drone2 = T::new(
        12,
        unbounded().0,
        d_command_recv.clone(),
        d2_recv.clone(),
        neighbours,
        0.0,
    );

    thread::spawn(move || {
        drone2.run();
    });

    let neighbours = HashMap::from([(11, d_send.clone())]);
    let mut drone3 = T::new(
        13,
        unbounded().0,
        d_command_recv.clone(),
        d3_recv.clone(),
        neighbours,
        0.0,
    );

    thread::spawn(move || {
        drone3.run();
    });

    let mut msg = create_sample_flood_req(1);
    // Client sends packet to d
    d_send.send(msg.clone()).unwrap();

    let f_res12 = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse {
            flood_id: 1,
            path_trace: vec![
                (1, NodeType::Client),
                (11, NodeType::Drone),
                (12, NodeType::Drone),
            ],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 2,
            hops: vec![12, 11, 1],
        },
        session_id: 1,
    };

    let f_res13 = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse {
            flood_id: 1,
            path_trace: vec![
                (1, NodeType::Client),
                (11, NodeType::Drone),
                (13, NodeType::Drone),
            ],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 2,
            hops: vec![13, 11, 1],
        },
        session_id: 1,
    };

    // d2 and d3 receive a flood request from d (containing the path trace)
    let res = c_recv.recv().unwrap();
    assert!(
        res == f_res12 || res == f_res13,
        "assertion `left == right` failed:\nleft: `{:?}`\nright1: `{:?}`\nright2: `{:?}`",
        res,
        f_res12,
        f_res13
    );
    let res = c_recv.recv().unwrap();
    assert!(
        res == f_res12 || res == f_res13,
        "assertion `left == right` failed:\nleft: `{:?}`\nright1: `{:?}`\nright2: `{:?}`",
        res,
        f_res12,
        f_res13
    );
}

// TODO known flood request behaviour
