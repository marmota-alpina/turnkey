//! Turnkey emulator crate providing device emulation functionality.
//!
//! This crate contains the state machine and logic for emulating
//! physical access control devices like turnstiles.

pub mod config;
pub mod display;
pub mod emulator;
pub mod handle;
pub mod state_machine;

pub use config::{EmulatorConfig, NetworkConfig, OperationMode};
pub use display::{Alignment, VirtualDisplay, VirtualDisplayBuilder, align_text, truncate_text};
pub use emulator::TurnstileEmulator;
pub use handle::{
    DisplayState, EVENT_CHANNEL_CAPACITY, EmulatorEvent, EmulatorHandle, ErrorCategory,
    INPUT_CHANNEL_CAPACITY, InputCommand,
};
pub use state_machine::{StateMachine, StateMachineBuilder, StateTransition};

// Re-export TurnstileState from protocol crate (single source of truth)
pub use turnkey_protocol::commands::turnstile::TurnstileState;
