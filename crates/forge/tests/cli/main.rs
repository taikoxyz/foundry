#![cfg_attr(foundry_network_restricted, allow(unused_imports))]

#[macro_use]
extern crate foundry_test_utils;

pub mod constants;
pub mod utils;

#[cfg(not(foundry_network_restricted))]
mod bind_json;
#[cfg(not(foundry_network_restricted))]
mod build;
#[cfg(not(foundry_network_restricted))]
mod cache;
#[cfg(not(foundry_network_restricted))]
mod cmd;
#[cfg(not(foundry_network_restricted))]
mod config;
#[cfg(not(foundry_network_restricted))]
mod context;
#[cfg(not(foundry_network_restricted))]
mod coverage;
#[cfg(not(foundry_network_restricted))]
mod create;
#[cfg(not(foundry_network_restricted))]
mod debug;
#[cfg(not(foundry_network_restricted))]
mod doc;
#[cfg(not(foundry_network_restricted))]
mod multi_script;
#[cfg(not(foundry_network_restricted))]
mod script;
#[cfg(not(foundry_network_restricted))]
mod soldeer;
#[cfg(not(foundry_network_restricted))]
mod svm;
#[cfg(not(foundry_network_restricted))]
mod test_cmd;
#[cfg(not(foundry_network_restricted))]
mod verify;
#[cfg(not(foundry_network_restricted))]
mod verify_bytecode;

#[cfg(not(foundry_network_restricted))]
mod ext_integration;

#[cfg(not(foundry_network_restricted))]
#[ctor::ctor]
fn skip_if_network_restricted() {
    if std::net::TcpListener::bind(("127.0.0.1", 0)).is_err() {
        eprintln!("forge CLI tests skipped: network access unavailable");
        std::process::exit(0);
    }
}
