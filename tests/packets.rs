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
    
    #[test]
    fn packet_receive() {
        // drone 1 <Packet>
        let (d_send, d_recv) = unbounded();
        
        let mut drone = RustezeDrone::new(DroneOptions{
            id: 1,
            pdr: 0.5,
            packet_send: HashMap::new(),
            packet_recv: d_recv.clone(),
            controller_send: unbounded().0,
            controller_recv: unbounded().1,
        });

        drone.run();

        let fragment = Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 1,
            data: [1; 80],
        };
        let srh = SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2, 3],
        };
        let msg = Packet { 
            pack_type: PacketType::MsgFragment(fragment), 
            routing_header: srh, 
            session_id: 1, 
        };
        d_send.send(msg.clone()).unwrap();

        assert_eq!(d_recv.recv().unwrap(), msg);
    }



