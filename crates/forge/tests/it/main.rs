pub mod config;
pub mod test_helpers;

#[cfg(not(foundry_network_restricted))]
mod cheats;
#[cfg(not(foundry_network_restricted))]
mod core;
#[cfg(not(foundry_network_restricted))]
mod fork;
#[cfg(not(foundry_network_restricted))]
mod fs;
#[cfg(not(foundry_network_restricted))]
mod fuzz;
#[cfg(not(foundry_network_restricted))]
mod inline;
#[cfg(not(foundry_network_restricted))]
mod invariant;
#[cfg(not(foundry_network_restricted))]
mod repros;
#[cfg(not(foundry_network_restricted))]
mod spec;
#[cfg(not(foundry_network_restricted))]
mod vyper;

#[cfg(not(foundry_network_restricted))]
#[ctor::ctor]
fn skip_if_network_restricted() {
    if std::net::TcpListener::bind(("127.0.0.1", 0)).is_err() {
        eprintln!("forge integration tests skipped: network access unavailable");
        std::process::exit(0);
    }
}
