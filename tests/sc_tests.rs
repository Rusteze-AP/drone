mod generic_fn;
use generic_fn::sc_generics::*;

use rusteze_drone::RustezeDrone;

#[test]
fn send_command() {
    generic_receive_sc_command::<RustezeDrone>();
}

// #[test]
// fn new_flood_neighbours() {
//     generic_new_flood_neighbours::<RustezeDrone>();
// }
