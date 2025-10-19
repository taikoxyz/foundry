#[cfg(not(foundry_network_restricted))]
mod abi;
#[cfg(not(foundry_network_restricted))]
mod anvil;
#[cfg(not(foundry_network_restricted))]
mod anvil_api;
#[cfg(not(foundry_network_restricted))]
mod api;
#[cfg(not(foundry_network_restricted))]
mod eip4844;
#[cfg(not(foundry_network_restricted))]
mod eip7702;
#[cfg(not(foundry_network_restricted))]
mod fork;
#[cfg(not(foundry_network_restricted))]
mod gas;
#[cfg(not(foundry_network_restricted))]
mod genesis;
#[cfg(not(foundry_network_restricted))]
mod ipc;
#[cfg(not(foundry_network_restricted))]
mod logs;
#[cfg(not(foundry_network_restricted))]
mod optimism;
#[cfg(not(foundry_network_restricted))]
mod otterscan;
#[cfg(not(foundry_network_restricted))]
mod proof;
#[cfg(not(foundry_network_restricted))]
mod pubsub;
#[cfg(not(foundry_network_restricted))]
mod revert;
#[cfg(not(foundry_network_restricted))]
mod sign;
#[cfg(not(foundry_network_restricted))]
mod state;
#[cfg(not(foundry_network_restricted))]
mod traces;
#[cfg(not(foundry_network_restricted))]
mod transaction;
#[cfg(not(foundry_network_restricted))]
mod txpool;
#[cfg(not(foundry_network_restricted))]
pub mod utils;
#[cfg(not(foundry_network_restricted))]
mod wsapi;

#[allow(unused)]
pub(crate) fn init_tracing() {
    let _ = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

#[cfg(not(foundry_network_restricted))]
fn main() {}

#[cfg(foundry_network_restricted)]
fn main() {
    eprintln!("anvil integration tests skipped: network access unavailable");
}
