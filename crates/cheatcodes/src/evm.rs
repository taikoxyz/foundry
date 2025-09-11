//! Implementations of [`Evm`](spec::Group::Evm) cheatcodes.

use crate::{
    BroadcastableTransaction, Cheatcode, Cheatcodes, CheatcodesExecutor, CheatsCtxt, Result, Vm::*,
};
use revm::{context_interface::JournalTr, context_interface::block::Block};
use alloy_consensus::TxEnvelope;
use alloy_genesis::{Genesis, GenesisAccount};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rlp::Decodable;
use alloy_sol_types::SolValue;
use foundry_common::fs::{read_json_file, write_json_file};
use foundry_evm_core::{
    backend::{DatabaseExt, RevertStateSnapshotAction},
    constants::{CALLER, CHEATCODE_ADDRESS, HARDHAT_CONSOLE_ADDRESS, TEST_CONTRACT_ADDRESS},
    env::{Env, EnvMut, EvmEnv},
};
use rand::Rng;
use revm::{
    primitives::{ChainAddress, KECCAK_EMPTY, hardfork::SpecId},
    state::{Account, Bytecode},
};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

mod fork;
pub(crate) mod mapping;
pub(crate) mod mock;
pub(crate) mod prank;

/// Records storage slots reads and writes.
#[derive(Clone, Debug, Default)]
pub struct RecordAccess {
    /// Storage slots reads.
    pub reads: HashMap<Address, Vec<U256>>,
    /// Storage slots writes.
    pub writes: HashMap<Address, Vec<U256>>,
}

impl RecordAccess {
    /// Records a read access to a storage slot.
    pub fn record_read(&mut self, target: Address, slot: U256) {
        self.reads.entry(target).or_default().push(slot);
    }

    /// Records a write access to a storage slot.
    ///
    /// This also records a read internally as `SSTORE` does an implicit `SLOAD`.
    pub fn record_write(&mut self, target: Address, slot: U256) {
        self.record_read(target, slot);
        self.writes.entry(target).or_default().push(slot);
    }
}

/// Records `deal` cheatcodes
#[derive(Clone, Debug)]
pub struct DealRecord {
    /// Target of the deal.
    pub address: ChainAddress,
    /// The balance of the address before deal was applied
    pub old_balance: U256,
    /// Balance after deal was applied
    pub new_balance: U256,
}

impl Cheatcode for addrCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        let Self { privateKey } = self;
        let wallet = super::crypto::parse_wallet(privateKey)?;
        Ok(wallet.address().abi_encode())
    }
}

impl Cheatcode for getNonce_0Call {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { account } = self;
        let chain_id = ccx.caller.0;
        get_nonce(ccx, &ChainAddress(chain_id, *account))
    }
}

impl Cheatcode for getNonce_1Call {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { wallet } = self;
        let chain_id = ccx.caller.0;
        get_nonce(ccx, &ChainAddress(chain_id, wallet.addr))
    }
}

impl Cheatcode for loadCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { target, slot } = *self;
        let chain_id = ccx.caller.0;
        let target = ChainAddress(chain_id, target);
        ensure_not_precompile!(target, ccx);
        ccx.ecx.journaled_state.load_account(target)?;
        let mut val = ccx.ecx.journaled_state.sload(target, slot.into())?;

        if val.is_cold && val.data.is_zero() {
            if ccx.state.arbitrary_storage.is_arbitrary(&target) {
                // If storage slot is untouched and load from a target with arbitrary storage,
                // then set random value for current slot.
                let rand_value = ccx.state.rng().r#gen();
                ccx.state.arbitrary_storage.save(ccx.ecx, target, slot.into(), rand_value);
                val.data = rand_value;
            } else if ccx.state.arbitrary_storage.is_copy(&target) {
                // If storage slot is untouched and load from a target that copies storage from
                // a source address with arbitrary storage, then copy existing arbitrary value.
                // If no arbitrary value generated yet, then the random one is saved and set.
                let rand_value = ccx.state.rng().r#gen();
                val.data =
                    ccx.state.arbitrary_storage.copy(ccx.ecx, target, slot.into(), rand_value);
            }
        }

        Ok(val.abi_encode())
    }
}

impl Cheatcode for loadAllocsCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { pathToAllocsJson } = self;

        let path = Path::new(pathToAllocsJson);
        ensure!(path.exists(), "allocs file does not exist: {pathToAllocsJson}");

        // Let's first assume we're reading a file with only the allocs.
        let allocs: BTreeMap<Address, GenesisAccount> = match read_json_file(path) {
            Ok(allocs) => allocs,
            Err(_) => {
                // Let's try and read from a genesis file, and extract allocs.
                let genesis = read_json_file::<Genesis>(path)?;
                genesis.alloc
            }
        };

        let chain_id = ccx.caller.0;
        let allocs = allocs.into_iter().map(|alloc| {
            (ChainAddress(chain_id, alloc.0), alloc.1)
        }).collect();

        // Then, load the allocs into the database.
        ccx.ecx.journaled_state.database
            .load_allocs(&allocs, &mut ccx.ecx.journaled_state.inner)
            .map(|()| Vec::default())
            .map_err(|e| fmt_err!("failed to load allocs: {e}"))
    }
}

impl Cheatcode for dumpStateCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { pathToStateJson } = self;
        let path = Path::new(pathToStateJson);

        // Do not include system account or empty accounts in the dump.
        let skip = |key: &Address, val: &Account| {
            key == &CHEATCODE_ADDRESS ||
                key == &CALLER ||
                key == &HARDHAT_CONSOLE_ADDRESS ||
                key == &TEST_CONTRACT_ADDRESS ||
                key == &ccx.caller.1 ||
                key == &ccx.state.config.evm_opts.sender ||
                val.is_empty()
        };

        let alloc = ccx
            .ecx
            .journaled_state
            .inner
            .state
            .iter_mut()
            .filter(|(key, val)| !skip(&key.1, val))
            .map(|(key, val)| {
                (
                    key,
                    GenesisAccount {
                        nonce: Some(val.info.nonce),
                        balance: val.info.balance,
                        code: val.info.code.as_ref().map(|o| o.original_bytes()),
                        storage: Some(
                            val.storage
                                .iter()
                                .map(|(k, v)| (B256::from(*k), B256::from(v.present_value())))
                                .collect(),
                        ),
                        private_key: None,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        write_json_file(path, &alloc)?;
        Ok(Default::default())
    }
}

impl Cheatcode for recordCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.accesses = Some(Default::default());
        Ok(Default::default())
    }
}

impl Cheatcode for accessesCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self { target } = *self;
        let result = state
            .accesses
            .as_mut()
            .map(|accesses| {
                (
                    &accesses.reads.entry(target).or_default()[..],
                    &accesses.writes.entry(target).or_default()[..],
                )
            })
            .unwrap_or_default();
        Ok(result.abi_encode_params())
    }
}

impl Cheatcode for recordLogsCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.recorded_logs = Some(Default::default());
        Ok(Default::default())
    }
}

impl Cheatcode for getRecordedLogsCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        Ok(state.recorded_logs.replace(Default::default()).unwrap_or_default().abi_encode())
    }
}

impl Cheatcode for pauseGasMeteringCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.gas_metering.paused = true;
        Ok(Default::default())
    }
}

impl Cheatcode for resumeGasMeteringCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.gas_metering.resume();
        Ok(Default::default())
    }
}

impl Cheatcode for resetGasMeteringCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.gas_metering.reset();
        Ok(Default::default())
    }
}

impl Cheatcode for lastCallGasCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        let Some(last_call_gas) = &state.gas_metering.last_call_gas else {
            bail!("no external call was made yet");
        };
        Ok(last_call_gas.abi_encode())
    }
}

impl Cheatcode for chainIdCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newChainId } = self;
        ensure!(*newChainId <= U256::from(u64::MAX), "chain ID must be less than 2^64 - 1");
        ccx.ecx.cfg.chain_id = newChainId.to();
        Ok(Default::default())
    }
}

impl Cheatcode for coinbaseCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newCoinbase } = self;
        let chain_id = ccx.caller.0;
        ccx.ecx.block.get_mut(&chain_id).unwrap().beneficiary = ChainAddress(chain_id, *newCoinbase);
        Ok(Default::default())
    }
}

impl Cheatcode for difficultyCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newDifficulty } = self;
        ensure!(
            ccx.ecx.cfg.spec < SpecId::MERGE,
            "`difficulty` is not supported after the Paris hard fork, use `prevrandao` instead; \
             see EIP-4399: https://eips.ethereum.org/EIPS/eip-4399"
        );
        let chain_id = ccx.caller.0;
        ccx.ecx.block.get_mut(&chain_id).unwrap().difficulty = *newDifficulty;
        Ok(Default::default())
    }
}

impl Cheatcode for feeCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newBasefee } = self;
        let chain_id = ccx.caller.0;
        ccx.ecx.block.get_mut(&chain_id).unwrap().basefee = newBasefee.saturating_to::<u64>();
        Ok(Default::default())
    }
}

impl Cheatcode for prevrandao_0Call {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newPrevrandao } = self;
        ensure!(
            ccx.ecx.cfg.spec >= SpecId::MERGE,
            "`prevrandao` is not supported before the Paris hard fork, use `difficulty` instead; \
             see EIP-4399: https://eips.ethereum.org/EIPS/eip-4399"
        );
        let chain_id = ccx.caller.0;
        ccx.ecx.block.get_mut(&chain_id).unwrap().prevrandao = Some(*newPrevrandao);
        Ok(Default::default())
    }
}

impl Cheatcode for prevrandao_1Call {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newPrevrandao } = self;
        ensure!(
            ccx.ecx.cfg.spec >= SpecId::MERGE,
            "`prevrandao` is not supported before the Paris hard fork, use `difficulty` instead; \
             see EIP-4399: https://eips.ethereum.org/EIPS/eip-4399"
        );
        let chain_id = ccx.caller.0;
        ccx.ecx.block.get_mut(&chain_id).unwrap().prevrandao = Some((*newPrevrandao).into());
        Ok(Default::default())
    }
}

impl Cheatcode for blobhashesCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { hashes } = self;
        ensure!(
            ccx.ecx.cfg.spec >= SpecId::CANCUN,
            "`blobhashes` is not supported before the Cancun hard fork; \
             see EIP-4844: https://eips.ethereum.org/EIPS/eip-4844"
        );
        ccx.ecx.tx.blob_hashes.clone_from(hashes);
        Ok(Default::default())
    }
}

impl Cheatcode for getBlobhashesCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self {} = self;
        ensure!(
            ccx.ecx.cfg.spec >= SpecId::CANCUN,
            "`getBlobhashes` is not supported before the Cancun hard fork; \
             see EIP-4844: https://eips.ethereum.org/EIPS/eip-4844"
        );
        Ok(ccx.ecx.tx.blob_hashes.clone().abi_encode())
    }
}

impl Cheatcode for rollCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newHeight } = self;
        let chain_id = ccx.caller.0;
        ccx.ecx.block.get_mut(&chain_id).unwrap().number = newHeight.saturating_to::<u64>();
        Ok(Default::default())
    }
}

impl Cheatcode for getBlockNumberCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self {} = self;
        let chain_id = ccx.caller.0;
        Ok(ccx.ecx.block.get_mut(&chain_id).unwrap().number.abi_encode())
    }
}

impl Cheatcode for txGasPriceCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newGasPrice } = self;
        ccx.ecx.tx.gas_price = newGasPrice.saturating_to::<u128>();
        Ok(Default::default())
    }
}

impl Cheatcode for warpCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newTimestamp } = self;
        let chain_id = ccx.caller.0;
        ccx.ecx.block.get_mut(&chain_id).unwrap().timestamp = newTimestamp.saturating_to::<u64>();
        Ok(Default::default())
    }
}

impl Cheatcode for getBlockTimestampCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self {} = self;
        let chain_id = ccx.caller.0;
        Ok(ccx.ecx.block.get_mut(&chain_id).unwrap().timestamp.abi_encode())
    }
}

impl Cheatcode for blobBaseFeeCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { newBlobBaseFee } = self;
        ensure!(
            ccx.ecx.cfg.spec >= SpecId::CANCUN,
            "`blobBaseFee` is not supported before the Cancun hard fork; \
             see EIP-4844: https://eips.ethereum.org/EIPS/eip-4844"
        );
        let chain_id = ccx.caller.0;
        ccx.ecx.block.get_mut(&chain_id).unwrap().set_blob_excess_gas_and_price((*newBlobBaseFee).to(), ccx.ecx.cfg.spec.is_enabled_in(SpecId::PRAGUE));
        Ok(Default::default())
    }
}

impl Cheatcode for getBlobBaseFeeCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self {} = self;
        let chain_id = ccx.caller.0;
        Ok(ccx.ecx.block.get_mut(&chain_id).unwrap().blob_excess_gas().unwrap_or(0).abi_encode())
    }
}

impl Cheatcode for dealCall {
    fn apply_stateful<'a>(&self, ccx: &'a mut CheatsCtxt<'a, 'a>) -> Result {
        let Self { account: address, newBalance: new_balance } = *self;
        let chain_id = ccx.caller.0;
        let account = journaled_account(ccx.ecx, ChainAddress(chain_id, address))?;
        let old_balance = std::mem::replace(&mut account.info.balance, new_balance);
        let record = DealRecord { address: ChainAddress(chain_id, address), old_balance, new_balance };
        ccx.state.eth_deals.push(record);
        Ok(Default::default())
    }
}

impl Cheatcode for etchCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { target, newRuntimeBytecode } = self;
        let chain_id = ccx.caller.0;
        ensure_not_precompile!(ChainAddress(chain_id, *target), ccx);
        ccx.ecx.journaled_state.load_account(ChainAddress(chain_id, *target))?;
        let bytecode = Bytecode::new_raw(Bytes::copy_from_slice(newRuntimeBytecode));
        ccx.ecx.journaled_state.set_code(ChainAddress(chain_id, *target), bytecode);
        Ok(Default::default())
    }
}

impl Cheatcode for resetNonceCall {
    fn apply_stateful<'a>(&self, ccx: &'a mut CheatsCtxt<'a, 'a>) -> Result {
        let Self { account } = self;
        let chain_id = ccx.caller.0;
        let account = journaled_account(ccx.ecx, ChainAddress(chain_id, *account))?;
        // Per EIP-161, EOA nonces start at 0, but contract nonces
        // start at 1. Comparing by code_hash instead of code
        // to avoid hitting the case where account's code is None.
        let empty = account.info.code_hash == KECCAK_EMPTY;
        let nonce = if empty { 0 } else { 1 };
        account.info.nonce = nonce;
        debug!(target: "cheatcodes", nonce, "reset");
        Ok(Default::default())
    }
}

impl Cheatcode for setNonceCall {
    fn apply_stateful<'a>(&self, ccx: &'a mut CheatsCtxt<'a, 'a>) -> Result {
        let Self { account, newNonce } = *self;
        let chain_id = ccx.caller.0;
        let account = journaled_account(ccx.ecx, ChainAddress(chain_id, account))?;
        // nonce must increment only
        let current = account.info.nonce;
        ensure!(
            newNonce >= current,
            "new nonce ({newNonce}) must be strictly equal to or higher than the \
             account's current nonce ({current})"
        );
        account.info.nonce = newNonce;
        Ok(Default::default())
    }
}

impl Cheatcode for setNonceUnsafeCall {
    fn apply_stateful<'a>(&self, ccx: &'a mut CheatsCtxt<'a, 'a>) -> Result {
        let Self { account, newNonce } = *self;
        let chain_id = ccx.caller.0;
        let account = journaled_account(ccx.ecx, ChainAddress(chain_id, account))?;
        account.info.nonce = newNonce;
        Ok(Default::default())
    }
}

impl Cheatcode for storeCall {
    fn apply_stateful<'a>(&self, ccx: &'a mut CheatsCtxt<'a, 'a>) -> Result {
        let Self { target, slot, value } = *self;
        let chain_id = ccx.caller.0;
        let target_chain_addr = ChainAddress(chain_id, target);
        ensure_not_precompile!(target_chain_addr, ccx);
        // ensure the account is touched and store value
        ccx.ecx.journaled_state.load_account(target_chain_addr)?;
        ccx.ecx.journaled_state.sstore(target_chain_addr, slot.into(), value.into())?;
        Ok(Default::default())
    }
}

impl Cheatcode for coolCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { target } = self;
        let chain_id = ccx.caller.0;
        if let Some(account) = ccx.ecx.journaled_state.inner.state.get_mut(&ChainAddress(chain_id, *target)) {
            account.unmark_touch();
            account.storage.clear();
        }
        Ok(Default::default())
    }
}

impl Cheatcode for readCallersCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self {} = self;
        read_callers(ccx.state, &ccx.ecx.tx.caller.1)
    }
}

impl Cheatcode for snapshotCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self {} = self;
        Ok(ccx.ecx.journaled_state.database.snapshot_state(
            &ccx.ecx.journaled_state.inner, 
            &mut EnvMut {
                block: &mut ccx.ecx.block,
                cfg: &mut ccx.ecx.cfg,
                tx: &mut ccx.ecx.tx,
            }
        ).abi_encode())
    }
}

impl Cheatcode for revertToCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { snapshotId } = self;
        let result = if let Some(journaled_state) = ccx.ecx.journaled_state.database.revert_state(
            *snapshotId,
            &ccx.ecx.journaled_state.inner,
            &mut EnvMut {
                block: &mut ccx.ecx.block,
                cfg: &mut ccx.ecx.cfg,
                tx: &mut ccx.ecx.tx,
            },
            RevertStateSnapshotAction::RevertKeep,
        ) {
            // we reset the evm's journaled_state to the state of the snapshot previous state
            ccx.ecx.journaled_state.inner = journaled_state;
            true
        } else {
            false
        };
        Ok(result.abi_encode())
    }
}

impl Cheatcode for revertToAndDeleteCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { snapshotId } = self;
        let result = if let Some(journaled_state) = ccx.ecx.journaled_state.database.revert_state(
            *snapshotId,
            &ccx.ecx.journaled_state.inner,
            &mut EnvMut {
                block: &mut ccx.ecx.block,
                cfg: &mut ccx.ecx.cfg,
                tx: &mut ccx.ecx.tx,
            },
            RevertStateSnapshotAction::RevertRemove,
        ) {
            // we reset the evm's journaled_state to the state of the snapshot previous state
            ccx.ecx.journaled_state.inner = journaled_state;
            true
        } else {
            false
        };
        Ok(result.abi_encode())
    }
}

impl Cheatcode for deleteSnapshotCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { snapshotId } = self;
        let result = ccx.ecx.journaled_state.database.delete_state_snapshot(*snapshotId);
        Ok(result.abi_encode())
    }
}
impl Cheatcode for deleteSnapshotsCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self {} = self;
        ccx.ecx.journaled_state.database.delete_state_snapshots();
        Ok(Default::default())
    }
}

impl Cheatcode for startStateDiffRecordingCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.recorded_account_diffs_stack = Some(Default::default());
        Ok(Default::default())
    }
}

impl Cheatcode for stopAndReturnStateDiffCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        get_state_diff(state)
    }
}

impl Cheatcode for broadcastRawTransactionCall {
    fn apply_full<'a, E: CheatcodesExecutor>(
        &self,
        ccx: &'a mut CheatsCtxt<'a, 'a>,
        executor: &mut E,
    ) -> Result {
        let mut data = self.data.as_ref();
        let tx = TxEnvelope::decode(&mut data)
            .map_err(|err| fmt_err!("failed to decode RLP-encoded transaction: {err}"))?;

        ccx.ecx.journaled_state.database.transact_from_tx(
            &tx.clone().into(),
            Env {
                evm_env: EvmEnv {
                    block_env: ccx.ecx.block.clone(),
                    cfg_env: ccx.ecx.cfg.clone(),
                },
                tx: ccx.ecx.tx.clone(),
            },
            &mut ccx.ecx.journaled_state.inner,
            &mut executor.get_inspector(ccx.state),
        )?;

        if ccx.state.broadcast.is_some() {
            ccx.state.broadcastable_transactions.push_back(BroadcastableTransaction {
                rpc: ccx.ecx.journaled_state.database.active_fork_url(),
                transaction: tx.try_into()?,
            });
        }

        Ok(Default::default())
    }
}

impl Cheatcode for setBlockhashCall {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { blockNumber, blockHash } = *self;
        let chain_id = ccx.caller.0;
        ensure!(
            blockNumber <= U256::from(ccx.ecx.block.get_mut(&chain_id).unwrap().number),
            "block number must be less than or equal to the current block number"
        );

        ccx.ecx.journaled_state.database.set_blockhash(blockNumber, blockHash);

        Ok(Default::default())
    }
}

pub(super) fn get_nonce(ccx: &mut CheatsCtxt<'_, '_>, address: &ChainAddress) -> Result {
    let account = ccx.ecx.journaled_state.load_account(*address)?;
    Ok(account.info.nonce.abi_encode())
}

/// Reads the current caller information and returns the current [CallerMode], `msg.sender` and
/// `tx.origin`.
///
/// Depending on the current caller mode, one of the following results will be returned:
/// - If there is an active prank:
///     - caller_mode will be equal to:
///         - [CallerMode::Prank] if the prank has been set with `vm.prank(..)`.
///         - [CallerMode::RecurrentPrank] if the prank has been set with `vm.startPrank(..)`.
///     - `msg.sender` will be equal to the address set for the prank.
///     - `tx.origin` will be equal to the default sender address unless an alternative one has been
///       set when configuring the prank.
///
/// - If there is an active broadcast:
///     - caller_mode will be equal to:
///         - [CallerMode::Broadcast] if the broadcast has been set with `vm.broadcast(..)`.
///         - [CallerMode::RecurrentBroadcast] if the broadcast has been set with
///           `vm.startBroadcast(..)`.
///     - `msg.sender` and `tx.origin` will be equal to the address provided when setting the
///       broadcast.
///
/// - If no caller modification is active:
///     - caller_mode will be equal to [CallerMode::None],
///     - `msg.sender` and `tx.origin` will be equal to the default sender address.
fn read_callers(state: &Cheatcodes, default_sender: &Address) -> Result {
    let Cheatcodes { prank, broadcast, .. } = state;

    let mut mode = CallerMode::None;
    let mut new_caller = default_sender;
    let mut new_origin = default_sender;
    if let Some(prank) = prank {
        mode = if prank.single_call { CallerMode::Prank } else { CallerMode::RecurrentPrank };
        new_caller = &prank.new_caller.1;
        if let Some(new) = &prank.new_origin {
            new_origin = &new.1;
        }
    } else if let Some(broadcast) = broadcast {
        mode = if broadcast.single_call {
            CallerMode::Broadcast
        } else {
            CallerMode::RecurrentBroadcast
        };
        new_caller = &broadcast.new_origin.1;
        new_origin = &broadcast.new_origin.1;
    }

    Ok((mode, new_caller, new_origin).abi_encode_params())
}

/// Ensures the `Account` is loaded and touched.
pub(super) fn journaled_account<'a>(
    ecx: &'a mut alloy_evm::eth::EthEvmContext<&'a mut dyn foundry_evm_core::backend::MultiChainDatabaseExt>,
    addr: ChainAddress,
) -> Result<&'a mut Account> {
    ecx.journaled_state.load_account(addr)?;
    ecx.journaled_state.touch(addr);
    Ok(ecx.journaled_state.inner.state.get_mut(&addr).expect("account is loaded"))
}

/// Consumes recorded account accesses and returns them as an abi encoded
/// array of [AccountAccess]. If there are no accounts were
/// recorded as accessed, an abi encoded empty array is returned.
///
/// In the case where `stopAndReturnStateDiff` is called at a lower
/// depth than `startStateDiffRecording`, multiple `Vec<RecordedAccountAccesses>`
/// will be flattened, preserving the order of the accesses.
fn get_state_diff(state: &mut Cheatcodes) -> Result {
    let res = state
        .recorded_account_diffs_stack
        .replace(Default::default())
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok(res.abi_encode())
}

// TODO: Stub implementations for missing cheatcodes - need proper implementation
impl Cheatcode for startDebugTraceRecordingCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for stopAndReturnDebugTraceRecordingCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for cloneAccountCall {
    fn apply_stateful(&self, _ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for stopRecordCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getStateDiffCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        get_state_diff(state)
    }
}

impl Cheatcode for getStateDiffJsonCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        get_state_diff(state)
    }
}

impl Cheatcode for accessListCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for noAccessListCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for warmSlotCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for coolSlotCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for mockCallRevert_2Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for mockCallRevert_3Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for mockCall_2Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for mockCall_3Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for mockCalls_0Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for mockCalls_1Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for prank_2Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for prank_3Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for startPrank_2Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for startPrank_3Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for deleteStateSnapshotCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for deleteStateSnapshotsCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for foundryVersionAtLeastCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(true.abi_encode())
    }
}

impl Cheatcode for revertToStateAndDeleteCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for revertToStateCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for snapshotGasLastCall_0Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for snapshotGasLastCall_1Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for snapshotStateCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for snapshotValue_0Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for snapshotValue_1Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for startSnapshotGas_0Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for startSnapshotGas_1Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for stopSnapshotGas_0Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for stopSnapshotGas_1Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for stopSnapshotGas_2Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for deployCode_2Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for deployCode_3Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for deployCode_4Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for deployCode_5Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for deployCode_6Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for deployCode_7Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for foundryVersionCmpCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getArtifactPathByCodeCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getArtifactPathByDeployedCodeCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getBroadcastCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getBroadcasts_0Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getBroadcasts_1Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getDeployment_0Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getDeployment_1Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for getDeploymentsCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for rememberKeys_0Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}

impl Cheatcode for rememberKeys_1Call {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        Ok(Default::default())
    }
}
