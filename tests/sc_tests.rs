use rusteze_drone::RustezeDrone;
use rusteze_tests::sc_generics::*;

#[test]
fn send_command() {
    generic_receive_sc_command::<RustezeDrone>();
}

// #[test]
// fn new_flood_neighbours() {
//     generic_new_flood_neighbours::<RustezeDrone>();
// }
