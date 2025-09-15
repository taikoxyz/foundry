use alloy_evm::EvmEnv;
use foundry_evm::EnvMut;
use foundry_evm_core::AsEnvMut;
//use op_revm::OpTransaction;
use revm::{
    context::{BlockEnv, CfgEnv, TxEnv},
    primitives::HashMap,
};

/// Helper container type for [`EvmEnv`] and [`OpTransaction<TxEnd>`].
#[derive(Clone, Debug, Default)]
pub struct Env {
    pub evm_env: EvmEnv,
    pub tx: TxEnv,
}

/// Helper container type for [`EvmEnv`] and [`OpTransaction<TxEnv>`].
impl Env {
    pub fn new(cfg: CfgEnv, block: BlockEnv, tx: TxEnv) -> Self {
        let mut block_env_map = HashMap::default();
        block_env_map.insert(cfg.chain_id, block); // Use the chain ID from cfg
        Self { evm_env: EvmEnv { cfg_env: cfg, block_env: block_env_map }, tx }
    }
}

impl AsEnvMut for Env {
    fn as_env_mut(&mut self) -> EnvMut<'_> {
        EnvMut {
            block: &mut self.evm_env.block_env, // Now block_env is HashMap<u64, BlockEnv>
            cfg: &mut self.evm_env.cfg_env,
            tx: &mut self.tx,
        }
    }
}
