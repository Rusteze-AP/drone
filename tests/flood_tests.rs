mod generic_fn;
use generic_fn::flood_generics::*;

use rusteze_drone::RustezeDrone;

#[test]
fn new_flood_request() {
    generic_new_flood::<RustezeDrone>();
}

#[test]
fn new_flood_req_no_initiator() {
    generic_new_flood_no_initiator::<RustezeDrone>();
}

#[test]
fn new_flood_neighbours() {
    generic_new_flood_neighbours::<RustezeDrone>();
}

#[test]
fn test_flood_res() {
    generic_flood_res::<RustezeDrone>();
}

#[test]
fn test_known_flood_req() {
    generic_known_flood_req::<RustezeDrone>();
}
