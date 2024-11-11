mod drone;
mod messages;
use std::thread;
use crossbeam::channel::unbounded;
use std::time::Duration;

fn main() {
    let (s1, r1) = unbounded();
    let (s2, r2) = unbounded();

    let drone1 = drone::Drone::new(1, vec![2], r1, r2, vec![], s1.clone());

    thread::spawn(move || drone1.run());
    
    loop {
        thread::sleep(Duration::from_secs(2));
        let s = String::from("hiiii");
        s1.send(s).unwrap();
        thread::sleep(Duration::from_secs(2));
        let s = String::from("byeee");
        s2.send(s).unwrap();
    }
}
