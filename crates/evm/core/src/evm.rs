use std::ops::{Deref, DerefMut};

use crate::{Env, InspectorExt, backend::MultiChainDatabaseExt};
use alloy_evm::{Evm, EvmEnv, eth::EthEvmContext, precompiles::PrecompilesMap};
use alloy_primitives::Bytes;
use foundry_fork_db::DatabaseError;
use revm::{
    Context, Journal,
    context::{
        BlockEnv, CfgEnv, ContextTr, Evm as RevmEvm, JournalTr, LocalContext, TxEnv,
        result::{EVMError, HaltReason, ResultAndState},
    },
    handler::{EthFrame, FrameResult, Handler, MainnetHandler, instructions::EthInstructions},
    inspector::InspectorHandler,
    interpreter::{FrameInput, SharedMemory, interpreter::EthInterpreter, interpreter_action::FrameInit},
    primitives::{ChainAddress, HashMap, MultiChainTxKind, hardfork::SpecId},
    ExecuteEvm, InspectEvm, InspectSystemCallEvm, SystemCallEvm,
    context_interface::LocalContextTr,
};

/// Constructs a [`FoundryEvm`] with a mutable inspector reference.
pub fn new_evm_with_inspector<'i, 'db, I: InspectorExt + ?Sized>(
    db: &'db mut dyn MultiChainDatabaseExt,
    env: Env,
    inspector: &'i mut I,
) -> FoundryEvm<'db, &'i mut I> {
    let mut journal = Journal::new(db);
    journal.set_spec_id(env.evm_env.cfg_env.spec);

    let ctx = EthEvmContext {
        journaled_state: journal,
        block: env.evm_env.block_env,
        cfg: env.evm_env.cfg_env,
        tx: env.tx,
        chain: (),
        local: LocalContext::default(),
        error: Ok(()),
    };
    let spec = ctx.cfg.spec;
    let xchain_enabled = ctx.cfg.xchain;
    let mut evm = FoundryEvm {
        inner: RevmEvm::new_with_inspector(
            ctx,
            inspector,
            EthInstructions::default(),
            get_precompiles(spec, xchain_enabled),
        ),
        inspect: true,
    };

    inject_precompiles(&mut evm);
    evm
}

/// Constructs a [`FoundryEvm`] from an existing context and inspector reference.
pub fn new_evm_with_existing_context<'a>(
    ctx: EthEvmContext<&'a mut dyn MultiChainDatabaseExt>,
    inspector: &'a mut dyn InspectorExt,
) -> FoundryEvm<'a, &'a mut dyn InspectorExt> {
    let spec = ctx.cfg.spec;
    let xchain_enabled = ctx.cfg.xchain;

    let mut evm = FoundryEvm {
        inner: RevmEvm::new_with_inspector(
            ctx,
            inspector,
            EthInstructions::default(),
            get_precompiles(spec, xchain_enabled),
        ),
        inspect: true,
    };

    inject_precompiles(&mut evm);
    evm
}

/// Conditionally inject additional precompiles. Currently a no-op.
fn inject_precompiles(_evm: &mut FoundryEvm<'_, impl InspectorExt>) {
    // Placeholder for Odyssey-specific precompile injection.
}

/// Returns the configured precompiles for the given spec.
fn get_precompiles(_spec: SpecId, _xchain: bool) -> PrecompilesMap {
    PrecompilesMap::from(revm::handler::EthPrecompiles::default())
}

/// A thin wrapper around `revm`'s [`Evm`] with Foundry-specific behaviour.
#[derive(Debug)]
pub struct FoundryEvm<'db, I: InspectorExt> {
    #[allow(clippy::type_complexity)]
    pub(crate) inner: RevmEvm<
        EthEvmContext<&'db mut dyn MultiChainDatabaseExt>,
        I,
        EthInstructions<EthInterpreter, EthEvmContext<&'db mut dyn MultiChainDatabaseExt>>,
        PrecompilesMap,
        EthFrame<EthInterpreter>,
    >,
    inspect: bool,
}

impl<I: InspectorExt> FoundryEvm<'_, I> {
    /// Runs a single execution frame, returning the resulting [`FrameResult`].
    pub fn run_execution(
        &mut self,
        frame: FrameInput,
    ) -> Result<FrameResult, EVMError<DatabaseError>> {
        let mut handler: MainnetHandler<_, EVMError<DatabaseError>, EthFrame<EthInterpreter>> =
            MainnetHandler::default();

        let depth = self.inner.ctx.journal().depth();
        let memory =
            SharedMemory::new_with_buffer(self.inner.ctx.local().shared_memory_buffer().clone());
        let parent_chain_id = Some(self.inner.ctx.journal().current_chain_id());
        let parent_execution_mode = self.inner.ctx.journal().current_execution_mode();

        let frame_init = FrameInit {
            depth,
            memory,
            frame_input: frame,
            parent_chain_id,
            parent_execution_mode,
        };

        let mut frame_result =
            InspectorHandler::inspect_run_exec_loop(&mut handler, &mut self.inner, frame_init)?;
        Handler::last_frame_result(&mut handler, &mut self.inner, &mut frame_result)?;
        Ok(frame_result)
    }
}

impl<'db, I: InspectorExt> Evm for FoundryEvm<'db, I> {
    type Precompiles = PrecompilesMap;
    type Inspector = I;
    type DB = &'db mut dyn MultiChainDatabaseExt;
    type Error = EVMError<DatabaseError>;
    type HaltReason = HaltReason;
    type Spec = SpecId;
    type Tx = TxEnv;

    fn chain_id(&self) -> u64 {
        self.inner.ctx.cfg.chain_id
    }

    fn blocks(&self) -> &HashMap<u64, BlockEnv> {
        &self.inner.ctx.block
    }

    fn block(&self) -> &BlockEnv {
        let chain_id = self.chain_id();
        self.blocks()
            .get(&chain_id)
            .or_else(|| self.blocks().get(&0))
            .expect("No block environment found for chain or fallback chain 0")
    }

    fn transact_raw(
        &mut self,
        mut tx: Self::Tx,
    ) -> Result<ResultAndState<Self::HaltReason>, Self::Error> {
        // If no explicit chain id is provided, use the parent if available, otherwise default chain id.
        if tx.chain_id.is_none() {
            let default_chain_id = self
                .inner
                .ctx
                .cfg
                .parent_chain_id
                .unwrap_or(self.inner.ctx.cfg.chain_id);
            tx.chain_id = Some(default_chain_id);
            tx.caller = ChainAddress::new(default_chain_id, tx.caller.address());
            if let MultiChainTxKind::Call(ref mut addr) = tx.kind {
                *addr = ChainAddress::new(default_chain_id, addr.address());
            }
        }

        // Ensure the transaction knows about all chains present in the environment.
        if tx.chain_ids.is_none() {
            tx.chain_ids = Some(self.inner.ctx.block.keys().copied().collect());
        }

        if self.inspect {
            self.inner.inspect_tx(tx)
        } else {
            self.inner.transact(tx)
        }
    }

    fn transact_system_call(
        &mut self,
        caller: ChainAddress,
        contract: ChainAddress,
        data: Bytes,
    ) -> Result<ResultAndState<Self::HaltReason>, Self::Error> {
        if self.inspect {
            self.inner.inspect_system_call_with_caller(caller, contract, data)
        } else {
            self.inner.system_call_with_caller(caller, contract, data)
        }
    }

    fn db_mut(&mut self) -> &mut Self::DB {
        &mut self.inner.ctx.journaled_state.database
    }

    fn precompiles(&self) -> &Self::Precompiles {
        &self.inner.precompiles
    }

    fn precompiles_mut(&mut self) -> &mut Self::Precompiles {
        &mut self.inner.precompiles
    }

    fn inspector(&self) -> &Self::Inspector {
        &self.inner.inspector
    }

    fn inspector_mut(&mut self) -> &mut Self::Inspector {
        &mut self.inner.inspector
    }

    fn set_inspector_enabled(&mut self, enabled: bool) {
        self.inspect = enabled;
    }

    fn components(&self) -> (&Self::DB, &Self::Inspector, &Self::Precompiles) {
        (
            &self.inner.ctx.journaled_state.database,
            &self.inner.inspector,
            &self.inner.precompiles,
        )
    }

    fn components_mut(&mut self) -> (&mut Self::DB, &mut Self::Inspector, &mut Self::Precompiles) {
        (
            &mut self.inner.ctx.journaled_state.database,
            &mut self.inner.inspector,
            &mut self.inner.precompiles,
        )
    }

    fn finish(self) -> (Self::DB, EvmEnv<Self::Spec>)
    where
        Self: Sized,
    {
        let Context { block: block_env, cfg: cfg_env, journaled_state, .. } = self.inner.ctx;
        (journaled_state.database, EvmEnv { block_env, cfg_env })
    }
}

impl<'db, I: InspectorExt> Deref for FoundryEvm<'db, I> {
    type Target = Context<BlockEnv, TxEnv, CfgEnv, &'db mut dyn MultiChainDatabaseExt>;

    fn deref(&self) -> &Self::Target {
        &self.inner.ctx
    }
}

impl<I: InspectorExt> DerefMut for FoundryEvm<'_, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.ctx
    }
}
