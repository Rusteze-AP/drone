use crossbeam::channel::unbounded;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::drone::{Drone};
use wg_internal::network::SourceRoutingHeader;
use wg_internal::packet::{Fragment, Nack, NackType, Packet, PacketType};



const TIMEOUT: Duration = Duration::from_millis(400);

pub fn generic_receive_sc_command<T: Drone + Send + 'static>(){
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d2_send, d2_recv) = unbounded::<Packet>();
    // SC commands
    let (d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    let mut drone = T::new(
        11,
        d_event_send,
        d_command_recv.clone(),
        d_recv.clone(),
        HashMap::from([(12, d2_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    d_command_send.send(DroneCommand::Crash).unwrap(); // in this case a Crash command is sent to the drone
    assert_eq!(
        d_command_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneCommand::Crash
    );

}



pub fn generic_handle_crash<T: Drone + Send + 'static>(){
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d2_send, d2_recv) = unbounded::<Packet>();
    // SC commands
    let (d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    let mut drone11 = T::new(
        11,
        d_event_send,
        d_command_recv.clone(),
        d_recv.clone(),
        HashMap::from([(12, d2_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone11.run();
    });

    d_command_send.send(DroneCommand::Crash).unwrap(); // in this case a Crash command is sent to the drone
    assert_eq!(
        d_command_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneCommand::Crash
    );

}


