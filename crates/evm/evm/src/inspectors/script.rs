use alloy_primitives::{Address, Bytes};
use foundry_common::sh_err;
use foundry_evm_core::backend::DatabaseError;
use revm::{
    Database, Inspector,
    bytecode::opcode::ADDRESS,
    context::ContextTr,
    inspector::JournalExt,
    interpreter::{
        InstructionResult, Interpreter, InterpreterAction,
        interpreter::EthInterpreter,
        interpreter_types::{Jumps, LoopControl},
    },
    primitives::ChainAddress,
};

/// An inspector that enforces certain rules during script execution.
///
/// Currently, it only warns if the `ADDRESS` opcode is used within the script's main contract.
#[derive(Clone, Debug, Default)]
pub struct ScriptExecutionInspector {
    /// The address of the script contract being executed.
    pub script_address: Address,
}

impl<CTX, D> Inspector<CTX, EthInterpreter> for ScriptExecutionInspector
where
    D: Database<Error = DatabaseError>,
    CTX: ContextTr<Db = D>,
    CTX::Journal: JournalExt,
{
    #[inline]
    fn step(&mut self, interpreter: &mut Interpreter, _ecx: &mut CTX) {
        // Check if both target and bytecode address are the same as script contract address
        // (allow calling external libraries when bytecode address is different).
        if interpreter.bytecode.opcode() == ADDRESS
            && interpreter.input.target_address.1 == self.script_address
            && interpreter.input.bytecode_address
                == Some(ChainAddress(interpreter.input.target_address.0, self.script_address))
        {
            // Log the reason for revert
            let _ = sh_err!(
                "Usage of `address(this)` detected in script contract. Script contracts are ephemeral and their addresses should not be relied upon."
            );
            // Set the instruction result to Revert to stop execution
            let gas = interpreter.gas;
            interpreter.bytecode.set_action(InterpreterAction::new_return(
                InstructionResult::Revert,
                Bytes::new(),
                gas,
            ));
        }
        // Note: We don't return anything here as step returns void.
        // The original check explicitly signaled to continue; leaving the action unset keeps the
        // default behavior.
    }
}
