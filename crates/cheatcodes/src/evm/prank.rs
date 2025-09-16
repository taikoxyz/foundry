use crate::{Cheatcode, Cheatcodes, CheatsCtxt, Result, Vm::*};
use revm::primitives::ChainAddress;

/// Prank information.
#[derive(Clone, Debug, Default)]
pub struct Prank {
    /// Address of the contract that initiated the prank
    pub prank_caller: ChainAddress,
    /// Address of `tx.origin` when the prank was initiated
    pub prank_origin: ChainAddress,
    /// The address to assign to `msg.sender`
    pub new_caller: ChainAddress,
    /// The address to assign to `tx.origin`
    pub new_origin: Option<ChainAddress>,
    /// The depth at which the prank was called
    pub depth: u64,
    /// Whether the prank stops by itself after the next call
    pub single_call: bool,
    /// Whether the prank has been used yet (false if unused)
    pub used: bool,
}

impl Prank {
    /// Create a new prank.
    pub fn new(
        prank_caller: ChainAddress,
        prank_origin: ChainAddress,
        new_caller: ChainAddress,
        new_origin: Option<ChainAddress>,
        depth: u64,
        single_call: bool,
    ) -> Self {
        Self { prank_caller, prank_origin, new_caller, new_origin, depth, single_call, used: false }
    }

    /// Apply the prank by setting `used` to true iff it is false
    /// Only returns self in the case it is updated (first application)
    pub fn first_time_applied(&self) -> Option<Self> {
        if self.used {
            None
        } else {
            Some(Self { used: true, ..self.clone() })
        }
    }
}

impl Cheatcode for prank_0Call {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { msgSender } = self;
        let msg_sender = &ChainAddress(ccx.state.chain_id, *msgSender);
        prank(ccx, msg_sender, None, true)
    }
}

impl Cheatcode for startPrank_0Call {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { msgSender } = self;
        let msg_sender = &ChainAddress(ccx.state.chain_id, *msgSender);
        prank(ccx, msg_sender, None, false)
    }
}

impl Cheatcode for prank_1Call {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { msgSender, txOrigin } = self;
        let msg_sender = &ChainAddress(ccx.state.chain_id, *msgSender);
        let tx_origin = &ChainAddress(ccx.state.chain_id, *txOrigin);
        prank(ccx, msg_sender, Some(tx_origin), true)
    }
}

impl Cheatcode for startPrank_1Call {
    fn apply_stateful(&self, ccx: &mut CheatsCtxt<'_, '_>) -> Result {
        let Self { msgSender, txOrigin } = self;
        let msg_sender = &ChainAddress(ccx.state.chain_id, *msgSender);
        let tx_origin = &ChainAddress(ccx.state.chain_id, *txOrigin);
        prank(ccx, msg_sender, Some(tx_origin), false)
    }
}

impl Cheatcode for stopPrankCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.prank = None;
        Ok(Default::default())
    }
}

fn prank(
    ccx: &mut CheatsCtxt<'_, '_>,
    new_caller: &ChainAddress,
    new_origin: Option<&ChainAddress>,
    single_call: bool,
) -> Result {
    let prank = Prank::new(
        ccx.caller,
        ccx.ecx.tx.caller,
        *new_caller,
        new_origin.copied(),
        ccx.ecx.journaled_state.inner.depth as u64,
        single_call,
    );

    if let Some(Prank { used, single_call: current_single_call, .. }) = ccx.state.prank {
        ensure!(used, "cannot overwrite a prank until it is applied at least once");
        // This case can only fail if the user calls `vm.startPrank` and then `vm.prank` later on.
        // This should not be possible without first calling `stopPrank`
        ensure!(
            single_call == current_single_call,
            "cannot override an ongoing prank with a single vm.prank; \
             use vm.startPrank to override the current prank"
        );
    }

    ensure!(
        ccx.state.broadcast.is_none(),
        "cannot `prank` for a broadcasted transaction; \
         pass the desired `tx.origin` into the `broadcast` cheatcode call"
    );

    ccx.state.prank = Some(prank);
    Ok(Default::default())
}
