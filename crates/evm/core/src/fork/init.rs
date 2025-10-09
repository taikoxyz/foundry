use std::collections::HashMap;

use crate::utils::apply_chain_and_block_specific_env_changes;
use alloy_primitives::U256;
use alloy_provider::{
    network::{BlockResponse, HeaderResponse},
    Network, Provider,
};
use alloy_rpc_types::BlockNumberOrTag;
use alloy_transport::{Transport, TransportResult};
use eyre::WrapErr;
use foundry_common::NON_ARCHIVE_NODE_WARNING;

use revm::primitives::{BlockEnv, CfgEnv, ChainAddress, Env, TxEnv};

/// Initializes a REVM block environment based on a forked
/// ethereum provider.
pub async fn environment<N: Network, T: Transport + Clone, P: Provider<T, N>>(
    provider: &P,
    memory_limit: u64,
    gas_price: Option<u128>,
    override_chain_id: Option<u64>,
    pin_block: Option<u64>,
    origin: ChainAddress,
    disable_block_gas_limit: bool,
) -> eyre::Result<(Env, N::BlockResponse)> {
    //let provider_chain_id = provider.get_chain_id().await?;
    //println!("provider_chain_id: {:?}", provider_chain_id);

    // Get the parent chain id from the provider
    let parent_chain_id = match provider
        .client()
        .request_noparams::<String>("eth_getParentChainId")
        .await
    {
        Ok(res) => {
            let without_prefix = res.trim_start_matches("0x");
            // Parse as base 16
            let parent_chain_id = u64::from_str_radix(without_prefix, 16).expect("Invalid hex input");
            Some(parent_chain_id)
        }
        Err(err) => {
            println!("error getting parent chain id: {err:?}");
            None
        }
    };
    println!("parent_chain_id: {parent_chain_id:?}");

    if let Err(err) = provider
        .client()
        .request::<_, bool>("eth_setActiveChainId", (parent_chain_id,))
        .await
    {
        println!("failed to set active chain id: {err:?}");
    }

    let block_number = if let Some(pin_block) = pin_block {
        pin_block
    } else {
        provider.get_block_number().await.wrap_err("Failed to get latest block number")?
    };
    let (fork_gas_price, rpc_chain_id, block) = tokio::try_join!(
        provider.get_gas_price(),
        provider.get_chain_id(),
        provider.get_block_by_number(BlockNumberOrTag::Number(block_number), false)
    )?;
    let block = if let Some(block) = block {
        block
    } else {
        if let Ok(latest_block) = provider.get_block_number().await {
            // If the `eth_getBlockByNumber` call succeeds, but returns null instead of
            // the block, and the block number is less than equal the latest block, then
            // the user is forking from a non-archive node with an older block number.
            if block_number <= latest_block {
                error!("{NON_ARCHIVE_NODE_WARNING}");
            }
            eyre::bail!(
                "Failed to get block for block number: {}\nlatest block number: {}",
                block_number,
                latest_block
            );
        }
        eyre::bail!("Failed to get block for block number: {}", block_number)
    };

    println!("FORK!!!");

    let mut cfg = CfgEnv::default();
    cfg.chain_id = override_chain_id.unwrap_or(rpc_chain_id);
    cfg.memory_limit = memory_limit;
    cfg.limit_contract_code_size = Some(usize::MAX);
    // EIP-3607 rejects transactions from senders with deployed code.
    // If EIP-3607 is enabled it can cause issues during fuzz/invariant tests if the caller
    // is a contract. So we disable the check by default.
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = disable_block_gas_limit;
    cfg.xchain = true;
    cfg.allow_mocking = true;
    cfg.parent_chain_id = parent_chain_id;

    // Try to get supported chain IDs from the RPC server
    // If not available, use current chain ID and parent chain ID as defaults
    let chain_ids = {
        // Try to get chain IDs from RPC (this might be a custom method)
        let result: TransportResult<Vec<u64>> = provider
            .client()
            .request_noparams("eth_getSupportedChains")
            .await;

        if let Ok(chain_ids_list) = result {
            if !chain_ids_list.is_empty() {
                Some(chain_ids_list)
            } else {
                // Fallback to default chain IDs if empty
                let mut default_ids = vec![cfg.chain_id];
                if let Some(parent_id) = parent_chain_id {
                    if parent_id != cfg.chain_id {
                        default_ids.push(parent_id);
                    }
                }
                Some(default_ids)
            }
        } else {
            // If RPC doesn't support getting chain IDs, use defaults
            let mut default_ids = vec![cfg.chain_id];
            if let Some(parent_id) = parent_chain_id {
                if parent_id != cfg.chain_id {
                    default_ids.push(parent_id);
                }
            }
            println!("RPC doesn't support eth_getSupportedChains, using defaults: {default_ids:?}");
            Some(default_ids)
        }
    };
    println!("chain_ids: {chain_ids:?}");

    let mut blocks = HashMap::new();
    for &chain_id in chain_ids.as_ref().unwrap().iter() {
        blocks.insert(chain_id, BlockEnv {
            number: U256::from(block.header().number()),
            timestamp: U256::from(block.header().timestamp()),
            coinbase: ChainAddress(cfg.chain_id, block.header().coinbase()),
            difficulty: block.header().difficulty(),
            prevrandao: block.header().mix_hash(),
            basefee: U256::from(block.header().base_fee_per_gas().unwrap_or_default()),
            gas_limit: U256::from(block.header().gas_limit()),
            ..Default::default()
        });
    }

    let mut env = Env {
        cfg: cfg.clone(),
        blocks,
        tx: TxEnv {
            caller: origin,
            gas_price: U256::from(gas_price.unwrap_or(fork_gas_price)),
            chain_ids,
            gas_limit: block.header().gas_limit() as u64,
            ..Default::default()
        },
    };

    apply_chain_and_block_specific_env_changes::<N>(&mut env, &block);

    Ok((env, block))
}
