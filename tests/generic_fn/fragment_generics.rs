use crossbeam::channel::unbounded;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_internal::controller::DroneEvent;
use wg_internal::drone::Drone;
use wg_internal::network::SourceRoutingHeader;
use wg_internal::packet::{Fragment, Nack, NackType, Packet, PacketType};

/* THE FOLLOWING TESTS CHECKS IF YOUR DRONE IS HANDLING CORRECTLY PACKETS (FRAGMENT) */

const TIMEOUT: Duration = Duration::from_millis(400);

/// Creates a sample packet for testing purposes. For convenience, using 1-10 for clients, 11-20 for drones and 21-30 for servers
fn create_sample_packet(hop_index: usize, hops : Vec<u8>) -> Packet {
    Packet::new_fragment(
        SourceRoutingHeader {
            hop_index,
            hops,
        },
        1,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        },
    )
}

fn get_ack(hop_index : usize, hops : Vec<u8>) -> Packet {
    Packet::new_ack(
        SourceRoutingHeader {
            hop_index,
            hops,
        },
        1,
        1,
    )
}

fn get_nack(hop_index : usize, hops: Vec<u8>, nack_type : NackType) -> Packet {
    Packet::new_nack(
        SourceRoutingHeader {
            hop_index,
            hops,
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type,
        },
    )
}

/// This function is used to test the packet forward functionality of a drone.
/// The assert consists in checking if the "client" and "SC" receive the correct packet.
pub fn generic_fragment_forward<T: Drone + Send + 'static>() {
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d2_send, d2_recv) = unbounded::<Packet>();
    // SC commands
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    let mut drone = T::new(
        11,
        d_event_send,
        d_command_recv,
        d_recv.clone(),
        HashMap::from([(12, d2_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    let mut msg = create_sample_packet(1, vec![1, 11, 12, 21]);

    // "Client" sends packet to d
    d_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 2;

    // d2 receives packet from d1
    assert_eq!(d2_recv.recv_timeout(TIMEOUT).unwrap(), msg);
    // SC listen for event from the drone
    assert_eq!(
        d_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(msg)
    );
}

/// Checks if the packet is dropped by one drone. The assert consists in checking if the "client" and "SC" receive the correct packet.
pub fn generic_fragment_drop<T: Drone + Send + 'static>() {
    // Client 1
    let (c_send, c_recv) = unbounded();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d2_send, _d2_recv) = unbounded();
    // SC commands
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    let mut drone = T::new(
        11,
        d_event_send,
        d_command_recv,
        d_recv,
        HashMap::from([(12, d2_send.clone()), (1, c_send.clone())]),
        1.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    let msg = create_sample_packet(1, vec![1, 11, 12, 21]);

    // "Client" sends packet to the drone
    d_send.send(msg.clone()).unwrap();

    let nack_packet = get_nack(1, vec![11, 1], NackType::Dropped);

    // Client listens for packet from the drone (Dropped Nack)
    assert_eq!(c_recv.recv_timeout(TIMEOUT).unwrap(), nack_packet);
    // SC listen for event from the drone
    assert_eq!(
        d_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketDropped(msg)
    );
}

/// Checks if the packet is dropped by the second drone. The first drone has 0% PDR and the second one 100% PDR, otherwise the test will fail sometimes.
/// The assert is checking only the NACK received by the client (It does not care about the SC events).
pub fn generic_chain_fragment_drop<T: Drone + Send + 'static>() {
    // Client 1 channels
    let (c_send, c_recv) = unbounded();
    // Server 21 channels
    let (s_send, _s_recv) = unbounded();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();
    // SC - needed to not make the drone crash / send DroneEvents
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, _d_event_recv) = unbounded();

    // Drone 11
    let mut drone = T::new(
        11,
        d_event_send.clone(),
        d_command_recv.clone(),
        d_recv,
        HashMap::from([(12, d12_send.clone()), (1, c_send.clone())]),
        0.0,
    );
    // Drone 12
    let mut drone2 = T::new(
        12,
        d_event_send.clone(),
        d_command_recv.clone(),
        d12_recv,
        HashMap::from([(11, d_send.clone()), (21, s_send.clone())]),
        1.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    thread::spawn(move || {
        drone2.run();
    });

    let msg = create_sample_packet(1, vec![1, 11, 12, 21]);

    // "Client" sends packet to the drone
    d_send.send(msg.clone()).unwrap();

    // Client receives an NACK originated from 'd2'
    assert_eq!(
        c_recv.recv_timeout(TIMEOUT).unwrap(),
        Packet {
            pack_type: PacketType::Nack(Nack {
                fragment_index: 1,
                nack_type: NackType::Dropped,
            }),
            routing_header: SourceRoutingHeader {
                hop_index: 2,
                hops: vec![12, 11, 1],
            },
            session_id: 1,
        }
    );
}

/// Checks if the packet can reach its destination. Both drones must have 0% PDR, otherwise the test will fail sometimes.
/// The assert is checking only the ACK received by the client (It does not care about the SC events).
pub fn generic_chain_fragment_ack<T: Drone + Send + 'static>() {
    // Client 1
    let (c_send, c_recv) = unbounded();
    // Server 21
    let (s_send, s_recv) = unbounded();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();
    // SC - needed to not make the drone crash
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, _d_event_recv) = unbounded();

    // Drone 11
    let mut drone = T::new(
        11,
        d_event_send.clone(),
        d_command_recv.clone(),
        d_recv,
        HashMap::from([(12, d12_send.clone()), (1, c_send.clone())]),
        0.0,
    );
    // Drone 12
    let mut drone2 = T::new(
        12,
        d_event_send.clone(),
        d_command_recv.clone(),
        d12_recv,
        HashMap::from([(11, d_send.clone()), (21, s_send.clone())]),
        0.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    thread::spawn(move || {
        drone2.run();
    });

    let mut msg = create_sample_packet(1, vec![1, 11, 12, 21]);

    // "Client" sends packet to the drone
    d_send.send(msg.clone()).unwrap();

    msg.routing_header.hop_index = 3;
    // Server receives the fragment
    assert_eq!(s_recv.recv_timeout(TIMEOUT).unwrap(), msg);

    // Server sends an ACK
    d12_send
        .send(Packet::new_ack(
            SourceRoutingHeader {
                hop_index: 1,
                hops: vec![21, 12, 11, 1],
            },
            1,
            1,
        ))
        .unwrap();

    // Client receives an ACK originated from 's'
    assert_eq!(
        c_recv.recv_timeout(TIMEOUT).unwrap(),
        get_ack(3, vec![21, 12, 11, 1])
    );
}

/// Checks if the packet containing an ACK is correctly forwarded by the drone.
/// The assert consists in checking if the drone sends the packet to both the next drone and the SC.
pub fn generic_ack_forward<T: Drone + Send + 'static>() {
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();
    // SC - needed to not make the drone crash
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    // Drone 11
    let mut drone = T::new(
        11,
        d_event_send.clone(),
        d_command_recv.clone(),
        d_recv,
        HashMap::from([(12, d12_send.clone())]),
        0.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    let mut ack = get_ack(1, vec![1, 11, 12, 21]);

    // "Client" sends packet to d11
    d_send.send(ack.clone()).unwrap();
    ack.routing_header.hop_index = 2;

    // d12 receives packet from d11
    assert_eq!(d12_recv.recv_timeout(TIMEOUT).unwrap(), ack);
    // SC gets notified that the packet was sent by d11
    assert_eq!(
        d_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(ack)
    );
}

/// Checks if the packet containing an NACK is correctly forwarded by the drone.
/// The assert consists in checking if the drone sends the packet to both the next drone and the SC.
pub fn generic_nack_forward<T: Drone + Send + 'static>() {
        // Drone 11
        let (d_send, d_recv) = unbounded();
        // Drone 12
        let (d12_send, d12_recv) = unbounded();
        // SC - needed to not make the drone crash
        let (_d_command_send, d_command_recv) = unbounded();
        let (d_event_send, d_event_recv) = unbounded();
    
        // Drone 11
        let mut drone = T::new(
            11,
            d_event_send.clone(),
            d_command_recv.clone(),
            d_recv,
            HashMap::from([(12, d12_send.clone())]),
            0.0,
        );
    
        // Spawn the drone's run method in a separate thread
        thread::spawn(move || {
            drone.run();
        });
    
        let mut nack = get_nack(1, vec![1, 11, 12, 21], NackType::Dropped);
    
        // "Client" sends packet to d11
        d_send.send(nack.clone()).unwrap();
        nack.routing_header.hop_index = 2;
    
        // d12 receives packet from d11
        assert_eq!(d12_recv.recv_timeout(TIMEOUT).unwrap(), nack);
        // SC gets notified that the packet was sent by d11
        assert_eq!(
            d_event_recv.recv_timeout(TIMEOUT).unwrap(),
            DroneEvent::PacketSent(nack)
        );
}

pub fn generic_destination_is_drone<T: Drone + Send + 'static>() {
    // Client 1
    let (c_send, c_recv) = unbounded();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // SC - needed to not make the drone crash
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    // Drone 11
    let mut drone = T::new(
        11,
        d_event_send.clone(),
        d_command_recv.clone(),
        d_recv,
        HashMap::from([(1, c_send.clone())]),
        0.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    let packet = create_sample_packet(1, vec![1, 11]);

    // "Client" sends packet to d11
    d_send.send(packet.clone()).unwrap();

    // "Client" expects a NACK with DestionationIsDrone
    assert_eq!(
        c_recv.recv_timeout(TIMEOUT).unwrap(),
        get_nack(1, vec![11, 1], NackType::DestinationIsDrone)
    );
}

// pub fn generic_wrong_ack_receive<T: Drone + Send + 'static> () {
//     // Drone 11
//     let (d_send, d_recv) = unbounded();
//     // Drone 12
//     let (d12_send, d12_recv) = unbounded();
//     // SC - needed to not make the drone crash
//     let (_d_command_send, d_command_recv) = unbounded();
//     let (d_event_send, d_event_recv) = unbounded();

//     // Drone 11
//     let mut drone = T::new(
//         11,
//         d_event_send.clone(),
//         d_command_recv.clone(),
//         d_recv,
//         HashMap::from([(12, d12_send.clone())]),
//         0.0,
//     );

//     // Spawn the drone's run method in a separate thread
//     thread::spawn(move || {
//         drone.run();
//     });

//     let mut ack = get_ack(1, vec![21, 12, 11, 1]);

//     // "Client" sends packet to d11
//     d_send.send(ack.clone()).unwrap();
//     ack.routing_header.hop_index = 2;

//     // d12 receives packet from d11
//     assert_eq!(d12_recv.recv_timeout(TIMEOUT).unwrap(), ack);
//     // SC gets notified that the packet was sent by d11
//     assert_eq!(
//         d_event_recv.recv_timeout(TIMEOUT).unwrap(),
//         DroneEvent::PacketSent(ack)
//     );
// } 
