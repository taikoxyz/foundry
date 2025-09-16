use super::Ecx;
use crate::inspector::Cheatcodes;
use alloy_primitives::{Address, Bytes, U256};
use revm::interpreter::{CreateInputs, CreateScheme, EOFCreateInputs};
use revm::primitives::ChainAddress;

/// Common behaviour of legacy and EOF create inputs.
pub(crate) trait CommonCreateInput {
    fn caller(&self) -> ChainAddress;
    fn gas_limit(&self) -> u64;
    fn value(&self) -> U256;
    fn init_code(&self) -> Bytes;
    fn scheme(&self) -> Option<CreateScheme>;
    fn set_caller(&mut self, caller: ChainAddress);
    fn log_debug(&self, cheatcode: &mut Cheatcodes, scheme: &CreateScheme);
    fn allow_cheatcodes(&self, cheatcodes: &mut Cheatcodes, ecx: Ecx) -> ChainAddress;
}

impl CommonCreateInput for &mut CreateInputs {
    fn caller(&self) -> ChainAddress {
        self.caller
    }
    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }
    fn value(&self) -> U256 {
        self.value
    }
    fn init_code(&self) -> Bytes {
        self.init_code.clone()
    }
    fn scheme(&self) -> Option<CreateScheme> {
        Some(self.scheme)
    }
    fn set_caller(&mut self, caller: ChainAddress) {
        self.caller = caller;
    }
    fn log_debug(&self, cheatcode: &mut Cheatcodes, scheme: &CreateScheme) {
        let kind = match scheme {
            CreateScheme::Create => "create",
            CreateScheme::Create2 { .. } => "create2",
            CreateScheme::Custom { .. } => "custom",
        };
        debug!(target: "cheatcodes", tx=?cheatcode.broadcastable_transactions.back().unwrap(), "broadcastable {kind}");
    }
    fn allow_cheatcodes(&self, cheatcodes: &mut Cheatcodes, ecx: Ecx) -> ChainAddress {
        let old_nonce = ecx
            .journaled_state
            .inner
            .state
            .get(&self.caller)
            .map(|acc| acc.info.nonce)
            .unwrap_or_default();
        let created_address = self.created_address(old_nonce);
        let created_chain_address = ChainAddress(ecx.cfg.chain_id, created_address);
        // SAFETY: Transmute to bypass lifetime variance restrictions - ecx outlives this call
        let ecx_transmuted: &mut alloy_evm::eth::EthEvmContext<&mut dyn foundry_evm_core::backend::MultiChainDatabaseExt> = unsafe {
            std::mem::transmute(ecx)
        };
        cheatcodes.allow_cheatcodes_on_create(ecx_transmuted, self.caller, created_chain_address);
        created_chain_address
    }
}

impl CommonCreateInput for &mut EOFCreateInputs {
    fn caller(&self) -> ChainAddress {
        self.caller
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn init_code(&self) -> Bytes {
        // For EOF creates, we approximate with empty bytes since the initcode is in a different format
        Bytes::new()
    }

    fn scheme(&self) -> Option<CreateScheme> {
        None // EOF creates don't use traditional schemes
    }

    fn set_caller(&mut self, caller: ChainAddress) {
        self.caller = caller;
    }

    fn log_debug(&self, _cheatcodes: &mut Cheatcodes, _scheme: &CreateScheme) {
        let created_address = self.kind.created_address().unwrap_or(&Address::ZERO);
        println!("Create2Factory EOF create: {:?} -> {:?}", self.caller(), created_address);
    }

    fn allow_cheatcodes(&self, cheatcodes: &mut Cheatcodes, ecx: Ecx) -> ChainAddress {
        let created_address = *self.kind.created_address().unwrap_or(&Address::ZERO);
        let created_chain_address = ChainAddress(ecx.cfg.chain_id, created_address);
        // SAFETY: Transmute to bypass lifetime variance restrictions - ecx outlives this call
        let ecx_transmuted: &mut alloy_evm::eth::EthEvmContext<&mut dyn foundry_evm_core::backend::MultiChainDatabaseExt> = unsafe {
            std::mem::transmute(ecx)
        };
        cheatcodes.allow_cheatcodes_on_create(ecx_transmuted, self.caller, created_chain_address);
        created_chain_address
    }
}
