// mod generic_fn;
// use generic_fn::packet_generics::*;

use rusteze_drone::RustezeDrone;
use wg_internal::tests::*;

#[test]
fn fragment_forward() {
    generic_fragment_forward::<RustezeDrone>();
}

#[test]
fn fragment_drop() {
    generic_fragment_drop::<RustezeDrone>();
}

#[test]
fn drone_chain_fragment_drop() {
    generic_chain_fragment_drop::<RustezeDrone>();
}

#[test]
fn drone_chain_fragment_ack() {
    generic_chain_fragment_ack::<RustezeDrone>();
}
