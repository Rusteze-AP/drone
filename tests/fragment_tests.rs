mod generic_fn;
use generic_fn::fragment_generics::*;

use rusteze_drone::RustezeDrone;
// use wg_internal::tests::*;

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

#[test]
fn ack_forward() {
    generic_ack_forward::<RustezeDrone>();
}

#[test]
fn nack_forward() {
    generic_nack_forward::<RustezeDrone>();
}

#[test]
fn destination_is_drone() {
    generic_destination_is_drone::<RustezeDrone>();
}
