pub use alloy_evm::EvmEnv;
use revm::{
    Context, Journal, JournalEntry,
    context::{BlockEnv, CfgEnv, JournalInner, JournalTr, TxEnv},
    database_interface::MultiChainDatabase,
    primitives::{HashMap, hardfork::SpecId},
};

/// Helper container type for [`EvmEnv`] and [`TxEnv`].
#[derive(Clone, Debug)]
pub struct Env {
    pub evm_env: EvmEnv,
    pub tx: TxEnv,
}

impl Default for Env {
    fn default() -> Self {
        let mut cfg = CfgEnv::default();
        cfg.chain_id = 31337;

        let mut tx = TxEnv::default();
        tx.chain_id = Some(31337);

        Self::from(cfg, BlockEnv::default(), tx)
    }
}
/// Helper container type for [`EvmEnv`] and [`TxEnv`].
impl Env {
    pub fn default_with_spec_id(spec_id: SpecId) -> Self {
        let mut cfg = CfgEnv::default();
        cfg.chain_id = 31337;
        cfg.spec = spec_id;

        let mut tx = TxEnv::default();
        tx.chain_id = Some(cfg.chain_id);

        Self::from(cfg, BlockEnv::default(), tx)
    }

    pub fn from(cfg: CfgEnv, block: BlockEnv, tx: TxEnv) -> Self {
        let mut blocks = HashMap::default();
        blocks.insert(cfg.chain_id, block);
        Self { evm_env: EvmEnv { cfg_env: cfg, block_env: blocks }, tx }
    }

    pub fn new_with_spec_id(cfg: CfgEnv, block: BlockEnv, tx: TxEnv, spec_id: SpecId) -> Self {
        let mut cfg = cfg;
        cfg.spec = spec_id;

        Self::from(cfg, block, tx)
    }
}

/// Helper struct with mutable references to the block and cfg environments.
pub struct EnvMut<'a> {
    pub block: &'a mut HashMap<u64, BlockEnv>,
    pub cfg: &'a mut CfgEnv,
    pub tx: &'a mut TxEnv,
}

impl EnvMut<'_> {
    /// Returns a copy of the environment.
    pub fn to_owned(&self) -> Env {
        Env {
            evm_env: EvmEnv { cfg_env: self.cfg.to_owned(), block_env: self.block.to_owned() },
            tx: self.tx.to_owned(),
        }
    }
}

pub trait AsEnvMut {
    fn as_env_mut(&mut self) -> EnvMut<'_>;
}

impl AsEnvMut for EnvMut<'_> {
    fn as_env_mut(&mut self) -> EnvMut<'_> {
        EnvMut { block: self.block, cfg: self.cfg, tx: self.tx }
    }
}

impl AsEnvMut for Env {
    fn as_env_mut(&mut self) -> EnvMut<'_> {
        EnvMut {
            block: &mut self.evm_env.block_env,
            cfg: &mut self.evm_env.cfg_env,
            tx: &mut self.tx,
        }
    }
}

impl<DB: MultiChainDatabase, J: JournalTr<Database = DB>, C> AsEnvMut
    for Context<BlockEnv, TxEnv, CfgEnv, DB, J, C>
{
    fn as_env_mut(&mut self) -> EnvMut<'_> {
        EnvMut { block: &mut self.block, cfg: &mut self.cfg, tx: &mut self.tx }
    }
}

pub trait ContextExt {
    type DB: crate::backend::MultiChainDatabaseExt;

    fn as_db_env_and_journal(
        &mut self,
    ) -> (&mut Self::DB, &mut JournalInner<JournalEntry>, EnvMut<'_>);
}

// this is right YSG
impl<DB: crate::backend::MultiChainDatabaseExt, C> ContextExt
    for Context<BlockEnv, TxEnv, CfgEnv, DB, Journal<DB, JournalEntry>, C>
{
    type DB = DB;

    fn as_db_env_and_journal(
        &mut self,
    ) -> (&mut Self::DB, &mut JournalInner<JournalEntry>, EnvMut<'_>) {
        (
            &mut self.journaled_state.database,
            &mut self.journaled_state.inner,
            EnvMut { block: &mut self.block, cfg: &mut self.cfg, tx: &mut self.tx },
        )
    }
}
