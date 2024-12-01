mod generic_fn;
use generic_fn::packet_generics::*;

use rusteze_drone::RustezeDrone;
// use wg_internal::tests::*;

#[test]
fn packet_forward() {
    generic_packet_forward::<RustezeDrone>();
}

#[test]
fn packet_drop() {
    generic_packet_drop::<RustezeDrone>();
}

#[test]
fn drone_chain_packet_drop() {
    generic_chain_packet_drop::<RustezeDrone>();
}

#[test]
fn drone_chain_packet_ack() {
    generic_chain_packet_ack::<RustezeDrone>();
}
