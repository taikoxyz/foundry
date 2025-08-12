//! The `forge` CLI: build, test, fuzz, debug and deploy Solidity contracts, like Hardhat, Brownie,
//! Ape.

use forge::args::run;
use backtrace::Backtrace;
use std::{
    panic::{self, PanicInfo},
    process, thread, time,
};

#[global_allocator]
static ALLOC: foundry_cli::utils::Allocator = foundry_cli::utils::new_allocator();


/// Invoke to ensure process exits on a thread panic.
pub fn setup_panic_handler() {
    panic::set_hook(Box::new(move |pi: &PanicInfo<'_>| {
        handle_panic(pi);
    }));
}

// Formats and logs panic information
fn handle_panic(panic_info: &PanicInfo<'_>) {
    let details = format!("{}", panic_info);
    let backtrace = format!("{:#?}", Backtrace::new());

    eprintln!("panic occurred:");
    eprintln!("details: {}", details);
    eprintln!("backtrace: {}", backtrace);

    // Provide some time to save the log to disk
    thread::sleep(time::Duration::from_millis(100));
    // Kill the process
    process::exit(12);
}

fn main() {
    setup_panic_handler();
    if let Err(err) = run() {
        let _ = foundry_common::sh_err!("{err:?}");
        std::process::exit(1);
    }
}
