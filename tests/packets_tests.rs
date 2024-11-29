// // drone 1 <Packet>
// let (d_send, d_recv) = unbounded();
// // SC <NodeEvent>
// let (sc_send, sc_recv) = unbounded();
// // drone 1 <DroneCommand> sender and receiver
// let (d_command_send, d_command_recv) = unbounded();
use std::collections::HashMap;
use rusteze_drone::RustezeDrone;
use wg_internal::drone::{self, Drone, DroneOptions};
use wg_internal::packet::Packet;
use crossbeam::channel::unbounded;

mod generic_fn;
use generic_fn::packet_generics::*;
 
// #[test]
// fn packet_receive() {
//     let (d_send, d_recv) = unbounded();
    
//     let drone = RustezeDrone::new(DroneOptions{
//         id: 1,
//         pdr: 0.0,
//         packet_send: HashMap::new(),
//         packet_recv: d_recv.clone(),
//         controller_send: unbounded().0, // we dont't care for this test
//         controller_recv: unbounded().1, // we dont't care for this test
//     });

//     generic_packet_receive(drone, &d_send, &d_recv);
// }

#[test]
fn packet_forward() {
    // drone 2 <Packet>
    let (d_send, d_recv) = unbounded();
    // drone 3 <Packet>
    let (d2_send, d2_recv) = unbounded::<Packet>();

    let (_d_command_send, d_command_recv) = unbounded();

    let neighbours = HashMap::from([(12, d2_send.clone())]);
    let mut drone = RustezeDrone::new(DroneOptions{
        id: 11,
        pdr: 0.0,
        packet_send: neighbours,
        packet_recv: d_recv.clone(),
        controller_send: unbounded().0,
        controller_recv: d_command_recv,
    });

    generic_packet_forward(drone, &d_send, &d2_recv);
}

#[test]
fn packet_drop() {
    let (c_send, c_recv) = unbounded();
    let (d_send, d_recv) = unbounded();
    let (_d_command_send, d_command_recv) = unbounded();

    let neighbours = HashMap::from([
        (12, d_send.clone()),
        (1, c_send.clone()),
        ]);
    let mut drone = RustezeDrone::new(DroneOptions{
        id: 11,
        pdr: 1.0,
        packet_send: neighbours,
        packet_recv: d_recv.clone(),
        controller_send: unbounded().0,
        controller_recv: d_command_recv,
    });

    generic_packet_drop(drone, &d_send, &c_recv);
}

#[test]
fn drone_chain_packet_drop() {
    // Client<1> channels
    let (c_send, c_recv) = unbounded();
    // Sever<21> channels
    let (s_send, _s_recv) = unbounded();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();
    // SC - needed to not make the drone crash
    let (_d_command_send, d_command_recv) = unbounded();

    // Drone 11
    let neighbours11 = HashMap::from([
        (12, d12_send.clone()),
        (1, c_send.clone()),
        ]);
    let mut drone = RustezeDrone::new(DroneOptions{
        id: 11,
        pdr: 0.0,
        packet_send: neighbours11,
        packet_recv: d_recv.clone(),
        controller_send: unbounded().0,
        controller_recv: d_command_recv.clone(),
    });
    // Drone 12
    let neighbours12 = HashMap::from([
        (11, d_send.clone()),
        (21, s_send.clone()),
        ]);
    let mut drone2 = RustezeDrone::new(DroneOptions{
        id: 12,
        pdr: 1.0,
        packet_send: neighbours12,
        packet_recv: d12_recv.clone(),
        controller_send: unbounded().0,
        controller_recv: d_command_recv.clone(),
    });

    generic_chain_packet_drop(drone, drone2, &d_send, &c_recv);
}
