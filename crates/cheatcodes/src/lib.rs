//! # foundry-cheatcodes
//!
//! Foundry cheatcodes implementations.

#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![allow(elided_lifetimes_in_paths)] // Cheats context uses 3 lifetimes

#[macro_use]
pub extern crate foundry_cheatcodes_spec as spec;
#[macro_use]
extern crate tracing;

use alloy_evm::eth::EthEvmContext;
use foundry_evm_core::backend::MultiChainDatabaseExt;
use revm::{primitives::ChainAddress};

pub use config::CheatsConfig;
pub use error::{Error, ErrorKind, Result};
pub use inspector::{
    BroadcastableTransaction, BroadcastableTransactions, Cheatcodes, CheatcodesExecutor, Context,
};
pub use spec::{CheatcodeDef, Vm};
pub use Vm::ForgeContext;

#[macro_use]
mod error;

mod base64;

mod config;

mod crypto;

mod env;
pub use env::set_execution_context;

mod evm;

mod fs;

mod inspector;

mod json;

mod script;
pub use script::{Wallets, WalletsInner};

mod string;

mod test;
pub use test::expect::ExpectedCallTracker;

mod toml;

mod utils;

/// Cheatcode implementation.
pub(crate) trait Cheatcode: CheatcodeDef + DynCheatcode {
    /// Applies this cheatcode to the given state.
    ///
    /// Implement this function if you don't need access to the EVM data.
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let _ = state;
        unimplemented!("{}", Self::CHEATCODE.func.id)
    }

    /// Applies this cheatcode to the given context.
    ///
    /// Implement this function if you need access to the EVM data.
    #[inline(always)]
    fn apply_stateful<'a>(&self, ccx: &'a mut CheatsCtxt<'a, 'a>) -> Result {
        self.apply(ccx.state)
    }

    /// Applies this cheatcode to the given context and executor.
    ///
    /// Implement this function if you need access to the executor.
    #[inline(always)]
    fn apply_full<'a, E: CheatcodesExecutor>(
        &self,
        ccx: &'a mut CheatsCtxt<'a, 'a>,
        _executor: &mut E,
    ) -> Result {
        self.apply_stateful(ccx)
    }
}

pub(crate) trait DynCheatcode {
    fn name(&self) -> &'static str;
    fn id(&self) -> &'static str;
    fn as_debug(&self) -> &dyn std::fmt::Debug;
}

impl<T: Cheatcode> DynCheatcode for T {
    fn name(&self) -> &'static str {
        T::CHEATCODE.func.signature.split('(').next().unwrap()
    }
    fn id(&self) -> &'static str {
        T::CHEATCODE.func.id
    }
    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}

/// The cheatcode context, used in `Cheatcode`.
pub struct CheatsCtxt<'cheats, 'evm> {
    /// The cheatcodes inspector state.
    pub(crate) state: &'cheats mut Cheatcodes,
    /// The EVM data.
    pub(crate) ecx: &'evm mut EthEvmContext<&'evm mut dyn MultiChainDatabaseExt>,
    /// The original `msg.sender`.
    pub(crate) caller: ChainAddress,
    /// Gas limit of the current cheatcode call.
    pub(crate) gas_limit: u64,
}

impl<'cheats, 'evm> std::ops::Deref for CheatsCtxt<'cheats, 'evm> {
    type Target = EthEvmContext<&'evm mut dyn MultiChainDatabaseExt>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.ecx
    }
}

impl<'cheats, 'evm> std::ops::DerefMut for CheatsCtxt<'cheats, 'evm> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ecx
    }
}

impl<'cheats, 'evm> CheatsCtxt<'cheats, 'evm> {
    /// Check if the given address is a precompile.
    pub fn is_precompile(&self, address: ChainAddress) -> bool {
        // Check if the address is in the precompile range
        // Precompiles are typically in the range 0x01 to 0x09 for Ethereum mainnet
        address.1.lt(&revm::primitives::Address::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10]))
    }
}
