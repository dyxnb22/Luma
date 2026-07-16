//! CLI exit codes and JSON outcome mapping for non-interactive commands.

use luma_protocol::ActionOutcomeDto;

/// Exit code for a one-shot action outcome.
pub fn action_exit_code(outcome: &ActionOutcomeDto) -> i32 {
    match outcome {
        ActionOutcomeDto::Success { .. } => 0,
        ActionOutcomeDto::Failed { .. } => 1,
        ActionOutcomeDto::Cancelled => 2,
        ActionOutcomeDto::InteractiveTerminal { .. } => 1,
    }
}
