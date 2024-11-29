use crossbeam::channel::{Receiver, Sender};
use std::thread;
use wg_internal::drone::Drone;
use wg_internal::network::SourceRoutingHeader;
use wg_internal::packet::{Ack, Fragment, Nack, NackType, Packet, PacketType};

fn create_sample_packet() -> Packet {
    let fragment = Fragment {
        fragment_index: 1,
        total_n_fragments: 1,
        length: 1,
        data: [1; 80],
    };
    // For convenience I am using 1-10 for clients, 11-20 for drones and 21-30 for servers
    let srh = SourceRoutingHeader {
        hop_index: 1,
        hops: vec![1, 11, 12, 21],
    };
    let msg = Packet {
        pack_type: PacketType::MsgFragment(fragment),
        routing_header: srh,
        session_id: 1,
    };
    msg
}

/// This function is used to test the packet forward functionality of a drone.
///
/// # Arguments
///
/// * `drone` - The drone instance that will be used to forward the packet.
/// * `d_send` - The Sender of the `drone` instance.
/// * `d2_recv` - The Receiver of the next drone in the `hops` vector.
pub fn generic_packet_forward<T: Drone + Send + 'static>(
    mut drone: T,
    d_send: &Sender<Packet>,
    d2_recv: &Receiver<Packet>,
) {
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    let mut msg = create_sample_packet();

    // "Client" sends packet to d
    d_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 2;

    // d2 receives packet from d1
    assert_eq!(d2_recv.recv().unwrap(), msg);
}

/// Checks if the packet is dropped by one drone. The drone MUST have 100% packet drop rate, otherwise the test will fail sometimes.
///
/// # Arguments
///
/// * `drone` - The drone instance with 100% PDR.
/// * `d_send` - The Sender of the `drone` instance.
/// * `c_recv` - The Receiver of the client.
pub fn generic_packet_drop<T: Drone + Send + 'static>(
    mut drone: T,
    d_send: &Sender<Packet>,
    c_recv: &Receiver<Packet>,
) {
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    let msg = create_sample_packet();

    // "Client" sends packet to the drone
    d_send.send(msg.clone()).unwrap();

    let dropped = Nack {
        fragment_index: 1,
        nack_type: NackType::Dropped,
    };
    let srh = SourceRoutingHeader {
        hop_index: 1,
        hops: vec![11, 1],
    };
    let nack_packet = Packet {
        pack_type: PacketType::Nack(dropped),
        routing_header: srh,
        session_id: 1,
    };

    // Client listens for packet from the drone (Dropped Nack)
    assert_eq!(c_recv.recv().unwrap(), nack_packet);
}

/// Checks if the packet is dropped by one drone. The first drone must have 0% PDR and the second one 100% PDR, otherwise the test will fail sometimes.
///
/// # Arguments
///
/// * `drone` - The drone instance with 0% PDR.
/// * `drone2` - The second drone instance in the chain with 100% PDR.
/// * `d_send` - The Sender of the `drone` instance.
/// * `c_recv` - The Receiver of the client.
pub fn generic_chain_packet_drop<T: Drone + Send + 'static>(
    mut drone: T,
    mut drone2: T,
    d_send: &Sender<Packet>,
    c_recv: &Receiver<Packet>,
) {
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    thread::spawn(move || {
        drone2.run();
    });

    let msg = create_sample_packet();

    // "Client" sends packet to the drone
    d_send.send(msg.clone()).unwrap();

    // Client receive an ACK originated from 'd'
    assert_eq!(
        c_recv.recv().unwrap(),
        Packet {
            pack_type: PacketType::Ack(Ack { fragment_index: 1 }),
            routing_header: SourceRoutingHeader {
                hop_index: 1,
                hops: vec![11, 1],
            },
            session_id: 1,
        }
    );

    // Client receive an NACK originated from 'd2'
    assert_eq!(
        c_recv.recv().unwrap(),
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
