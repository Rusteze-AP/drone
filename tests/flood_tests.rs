use rusteze_drone::RustezeDrone;
use rusteze_tests::flood_generics::*;

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
fn test_flood_res_forward() {
    generic_flood_res_forward::<RustezeDrone>();
}

#[test]
fn test_known_flood_req() {
    generic_known_flood_req::<RustezeDrone>();
}

#[test]
fn test_flood_req_two_initiator() {
    generic_flood_req_two_initiator::<RustezeDrone>();
}
