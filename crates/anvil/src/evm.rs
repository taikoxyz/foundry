use std::fmt::Debug;

use alloy_evm::{EthEvm, Evm, eth::EthEvmContext, precompiles::PrecompilesMap};
// use revm::Database; // Unused after multi-chain migration
//use foundry_evm_core::either_evm::EitherEvm;
//use op_revm::OpContext;
use revm::{Inspector, precompile::PrecompileWithAddress};

/// Object-safe trait that enables injecting extra precompiles when using
/// `anvil` as a library.
pub trait PrecompileFactory: Send + Sync + Unpin + Debug {
    /// Returns a set of precompiles to extend the EVM with.
    fn precompiles(&self) -> Vec<PrecompileWithAddress>;
}

/// Inject precompiles into the EVM dynamically.
pub fn inject_precompiles<DB, I>(
    evm: &mut EthEvm<DB, I, PrecompilesMap>,
    precompiles: Vec<PrecompileWithAddress>,
) where
    DB: alloy_evm::MultiDatabase,
    I: Inspector<EthEvmContext<DB>>,
{
    for p in precompiles {
        let precompile_fn = *p.precompile();
        evm.precompiles_mut().apply_precompile(p.address(), |_| {
            // We'll use unsafe to create DynPrecompile since the constructor is private
            // but this is the same pattern used in the alloy-evm codebase itself
            struct PrecompileWrapper {
                inner: fn(
                    &[u8],
                    u64,
                    &mut revm::precompile::PrecompileContext,
                ) -> revm::precompile::PrecompileResult,
            }

            impl alloy_evm::precompiles::Precompile for PrecompileWrapper {
                fn call(
                    &self,
                    data: &[u8],
                    gas: u64,
                    context: &mut revm::precompile::PrecompileContext,
                ) -> revm::precompile::PrecompileResult {
                    (self.inner)(data, gas, context)
                }
            }

            let _wrapper = PrecompileWrapper { inner: precompile_fn };

            // Unfortunately, we can't easily create DynPrecompile due to private constructor
            // For now, let's return None until we find another approach
            // TODO: Find a way to inject custom precompiles with the new alloy-evm API

            None
        });
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use alloy_evm::{EthEvm, Evm, EvmEnv, eth::EthEvmContext, precompiles::PrecompilesMap};
    //  use alloy_op_evm::OpEvm;
    use alloy_primitives::{Address, Bytes, address};
    //use foundry_evm_core::either_evm::EitherEvm;
    use itertools::Itertools;
    //use op_revm::{L1BlockInfo, OpContext, OpSpecId, OpTransaction, precompiles::OpPrecompiles};
    use revm::{
        Journal,
        context::{CfgEnv, Evm as RevmEvm, JournalTr, LocalContext, TxEnv},
        database::{EmptyDB, EmptyDBTyped},
        handler::{EthPrecompiles, instructions::EthInstructions},
        inspector::NoOpInspector,
        interpreter::interpreter::EthInterpreter,
        precompile::{
            PrecompileContext, PrecompileOutput, PrecompileResult, PrecompileSpecId,
            PrecompileWithAddress, Precompiles,
        },
        primitives::{ChainAddress, MultiChainTxKind, hardfork::SpecId},
    };

    use crate::{PrecompileFactory, inject_precompiles};

    // A precompile activated in the `Prague` spec.
    const ETH_PRAGUE_PRECOMPILE: Address = address!("0x0000000000000000000000000000000000000011");

    // A custom precompile address and payload for testing.
    const PRECOMPILE_ADDR: Address = address!("0x0000000000000000000000000000000000000071");
    const PAYLOAD: &[u8] = &[0xde, 0xad, 0xbe, 0xef];

    #[derive(Debug)]
    struct CustomPrecompileFactory;

    impl PrecompileFactory for CustomPrecompileFactory {
        fn precompiles(&self) -> Vec<PrecompileWithAddress> {
            vec![PrecompileWithAddress::from((
                PRECOMPILE_ADDR,
                custom_echo_precompile as fn(&[u8], u64, &mut PrecompileContext) -> PrecompileResult,
            ))]
        }
    }

    /// Custom precompile that echoes the input data.
    /// In this example it uses `0xdeadbeef` as the input data, returning it as output.
    fn custom_echo_precompile(
        input: &[u8],
        _gas_limit: u64,
        _context: &mut PrecompileContext,
    ) -> PrecompileResult {
        Ok(PrecompileOutput { bytes: Bytes::copy_from_slice(input), gas_used: 0 })
    }

    /// Creates a new EVM instance with the custom precompile factory.
    fn create_eth_evm(
        spec: SpecId,
    ) -> (foundry_evm::Env, EthEvm<EmptyDBTyped<Infallible>, NoOpInspector, PrecompilesMap>) {
        let eth_env = foundry_evm::Env {
            evm_env: EvmEnv { block_env: Default::default(), cfg_env: CfgEnv::new_with_spec(spec) },
            tx: TxEnv {
                kind: MultiChainTxKind::Call(ChainAddress(0, PRECOMPILE_ADDR)),
                data: PAYLOAD.into(),
                ..Default::default()
            },
        };

        let eth_evm_context = EthEvmContext {
            journaled_state: {
                let mut journal = Journal::new(EmptyDB::default());
                journal.set_spec_id(spec);
                journal.set_tx_origin_chain_id(0);
                journal.set_parent_chain_id(Some(0));
                journal
            },
            block: eth_env.evm_env.block_env.clone(),
            cfg: eth_env.evm_env.cfg_env.clone(),
            tx: eth_env.tx.clone(),
            chain: (),
            local: LocalContext::default(),
            error: Ok(()),
        };

        let eth_precompiles = EthPrecompiles {
            precompiles: Precompiles::new(PrecompileSpecId::from_spec_id(spec), false),
            spec,
            xchain: false,
        };
        let eth_evm = EthEvm::new(
            RevmEvm::new_with_inspector(
                eth_evm_context,
                NoOpInspector,
                EthInstructions::<EthInterpreter, EthEvmContext<EmptyDB>>::default(),
                PrecompilesMap::from_static(eth_precompiles.precompiles),
            ),
            true,
        );

        (eth_env, eth_evm)
    }

    /*
    /// Creates a new OP EVM instance with the custom precompile factory.
    fn create_op_evm(
        spec: SpecId,
        op_spec: OpSpecId,
    ) -> (
        crate::eth::backend::env::Env,
        EitherEvm<EmptyDBTyped<Infallible>, NoOpInspector, PrecompilesMap>,
    ) {
        let op_env = crate::eth::backend::env::Env {
            evm_env: EvmEnv { block_env: Default::default(), cfg_env: CfgEnv::new_with_spec(spec) },
            tx: OpTransaction::<TxEnv> {
                base: TxEnv {
                    kind: TxKind::Call(PRECOMPILE_ADDR),
                    data: PAYLOAD.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            is_optimism: true,
        };

        let mut chain = L1BlockInfo::default();

        if op_spec == OpSpecId::ISTHMUS {
            chain.operator_fee_constant = Some(U256::from(0));
            chain.operator_fee_scalar = Some(U256::from(0));
        }

        let op_cfg = op_env.evm_env.cfg_env.clone().with_spec(op_spec);
        let op_evm_context = OpContext {
            journaled_state: {
                let mut journal = Journal::new(EmptyDB::default());
                // Converting SpecId into OpSpecId
                journal.set_spec_id(op_env.evm_env.cfg_env.spec);
                journal.set_tx_origin_chain_id(0);
                journal.set_parent_chain_id(Some(0));
                journal
            },
            block: op_env.evm_env.block_env.clone(),
            cfg: op_cfg.clone(),
            tx: op_env.tx.clone(),
            chain,
            local: LocalContext::default(),
            error: Ok(()),
        };

        let op_precompiles = OpPrecompiles::new_with_spec(op_cfg.spec).precompiles();
        let op_evm = EitherEvm::Op(OpEvm::new(
            op_revm::OpEvm(RevmEvm::new_with_inspector(
                op_evm_context,
                NoOpInspector,
                EthInstructions::<EthInterpreter, OpContext<EmptyDB>>::default(),
                PrecompilesMap::from_static(op_precompiles),
            )),
            true,
        ));

        (op_env, op_evm)
    }
     */

    #[test]
    fn build_eth_evm_with_extra_precompiles_default_spec() {
        let (env, mut evm) = create_eth_evm(SpecId::default());

        // Check that the Prague precompile IS present when using the default spec.
        assert!(evm.precompiles().addresses().contains(&ETH_PRAGUE_PRECOMPILE));

        assert!(!evm.precompiles().addresses().contains(&PRECOMPILE_ADDR));

        inject_precompiles(&mut evm, CustomPrecompileFactory.precompiles());

        assert!(evm.precompiles().addresses().contains(&PRECOMPILE_ADDR));

        /*
        let result = match &mut evm {
            EitherEvm::Eth(eth_evm) => eth_evm.transact(env.tx).unwrap(),
            _ => unreachable!(),
        };
        */
        let result = evm.transact(env.tx).unwrap();
        assert!(result.result.is_success());
        assert_eq!(result.result.output(), Some(&PAYLOAD.into()));
    }

    #[test]
    fn build_eth_evm_with_extra_precompiles_london_spec() {
        let (env, mut evm) = create_eth_evm(SpecId::LONDON);

        // Check that the Prague precompile IS NOT present when using the London spec.
        assert!(!evm.precompiles().addresses().contains(&ETH_PRAGUE_PRECOMPILE));

        assert!(!evm.precompiles().addresses().contains(&PRECOMPILE_ADDR));

        inject_precompiles(&mut evm, CustomPrecompileFactory.precompiles());

        assert!(evm.precompiles().addresses().contains(&PRECOMPILE_ADDR));

        /*
        let result = match &mut evm {
            EitherEvm::Eth(eth_evm) => eth_evm.transact(env.tx).unwrap(),
            _ => unreachable!(),
        };
         */
        let result = evm.transact(env.tx).unwrap();
        assert!(result.result.is_success());
        assert_eq!(result.result.output(), Some(&PAYLOAD.into()));
    }

    /*
    #[test]
    fn build_op_evm_with_extra_precompiles_default_spec() {
        let (env, mut evm) = create_op_evm(SpecId::default(), OpSpecId::default());

        // Check that the Isthmus precompile IS present when using the default spec.
        assert!(evm.precompiles().addresses().contains(&OP_ISTHMUS_PRECOMPILE));

        // Check that the Prague precompile IS present when using the default spec.
        assert!(evm.precompiles().addresses().contains(&ETH_PRAGUE_PRECOMPILE));

        assert!(!evm.precompiles().addresses().contains(&PRECOMPILE_ADDR));

        inject_precompiles(&mut evm, CustomPrecompileFactory.precompiles());

        assert!(evm.precompiles().addresses().contains(&PRECOMPILE_ADDR));

        let result = match &mut evm {
            EitherEvm::Op(op_evm) => op_evm.transact(env.tx).unwrap(),
            _ => unreachable!(),
        };

        assert!(result.result.is_success());
        assert_eq!(result.result.output(), Some(&PAYLOAD.into()));
    }

    #[test]
    fn build_op_evm_with_extra_precompiles_bedrock_spec() {
        let (env, mut evm) = create_op_evm(SpecId::default(), OpSpecId::BEDROCK);

        // Check that the Isthmus precompile IS NOT present when using the `OpSpecId::BEDROCK` spec.
        assert!(!evm.precompiles().addresses().contains(&OP_ISTHMUS_PRECOMPILE));

        // Check that the Prague precompile IS NOT present when using the `OpSpecId::BEDROCK` spec.
        assert!(!evm.precompiles().addresses().contains(&ETH_PRAGUE_PRECOMPILE));

        assert!(!evm.precompiles().addresses().contains(&PRECOMPILE_ADDR));

        inject_precompiles(&mut evm, CustomPrecompileFactory.precompiles());

        assert!(evm.precompiles().addresses().contains(&PRECOMPILE_ADDR));

        let result = match &mut evm {
            EitherEvm::Op(op_evm) => op_evm.transact(env.tx).unwrap(),
            _ => unreachable!(),
        };

        assert!(result.result.is_success());
        assert_eq!(result.result.output(), Some(&PAYLOAD.into()));
    }
     */
}
