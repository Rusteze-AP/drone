// // drone 1 <Packet>
// let (d_send, d_recv) = unbounded();
// // SC <NodeEvent>
// let (sc_send, sc_recv) = unbounded();
// // drone 1 <DroneCommand> sender and receiver
// let (d_command_send, d_command_recv) = unbounded();
use std::collections::HashMap;
use rusteze_drone::RustezeDrone;
use wg_internal::drone::{Drone, DroneOptions};
use wg_internal::packet::{Fragment, Packet, PacketType};
use wg_internal::network::SourceRoutingHeader;
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::thread;
 
fn create_sample_packet() -> Packet {
    let fragment = Fragment {
        fragment_index: 1,
        total_n_fragments: 1,
        length: 1,
        data: [1; 80],
    };
    let srh = SourceRoutingHeader {
        hop_index: 1,
        hops: vec![1, 2, 3, 4],
    };
    let msg = Packet { 
        pack_type: PacketType::MsgFragment(fragment), 
        routing_header: srh, 
        session_id: 1, 
    };
    msg
}

#[test]
fn packet_receive() {
    // drone 1 <Packet>
    let (d_send, d_recv) = unbounded();
    
    let mut drone = RustezeDrone::new(DroneOptions{
        id: 1,
        pdr: 0.0,
        packet_send: HashMap::new(),
        packet_recv: d_recv.clone(),
        controller_send: unbounded().0, // we dont't care for this test
        controller_recv: unbounded().1, // we dont't care for this test
    });

    thread::spawn(move || {
        drone.run();
    });

    let msg = create_sample_packet();
    d_send.send(msg.clone()).unwrap();

    // Check if the packet is received correctly from the drone
    assert_eq!(d_recv.recv().unwrap(), msg);
}

#[test]
fn packet_forward() {
    // drone 1 <Packet>
    let (d2_send, d2_recv) = unbounded();
    // drone 2 <Packet>
    let (d3_send, d3_recv) = unbounded::<Packet>();

    let (d_command_send, d_command_recv) = unbounded();

    let neighbours = HashMap::from([(3, d3_send.clone())]);
    let mut drone = RustezeDrone::new(DroneOptions{
        id: 2,
        pdr: 0.0,
        packet_send: neighbours,
        packet_recv: d2_recv.clone(),
        controller_send: unbounded().0,
        controller_recv: d_command_recv.clone(),
    });

    thread::spawn(move || {
        drone.run();
    });

    let mut msg = create_sample_packet();

    d2_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 2;

    //println!("Received into drone {:?}", d2_recv.recv().unwrap());
    assert_eq!(d3_recv.recv().unwrap(), msg);
}


