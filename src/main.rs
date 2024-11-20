mod drone;
mod messages;
mod commons;
use crossbeam::channel::unbounded;
use messages::{Fragment, Packet, PacketType, SourceRoutingHeader, Ack, Nack};
use std::thread;
use std::time::Duration;


fn main() {
    let (s1, r1) = unbounded();
    let (s2, r2) = unbounded();

    let drone1 = drone::Drone::new(1, vec![2], r1, r2, vec![], s1.clone());

    thread::spawn(move || drone1.run());


    loop {
        let packet = Packet::new(PacketType::MsgFragment(Fragment::default()),SourceRoutingHeader::default(),0);
        s1.send(packet).unwrap();
        thread::sleep(Duration::from_secs(2));
        let packet = Packet::new(PacketType::Ack(Ack::default()),SourceRoutingHeader::default(),0);
        s1.send(packet).unwrap();
        thread::sleep(Duration::from_secs(2));
        let packet = Packet::new(PacketType::Nack(Nack::default()),SourceRoutingHeader::default(),0);
        s1.send(packet).unwrap();
        thread::sleep(Duration::from_secs(2));
    }
}
