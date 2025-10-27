//! Turnkey emulator crate providing device emulation functionality.
//!
//! This crate contains the state machine and logic for emulating
//! physical access control devices like turnstiles.

pub mod display;
pub mod state_machine;

pub use display::{Alignment, VirtualDisplay, VirtualDisplayBuilder, align_text, truncate_text};
pub use state_machine::{StateMachine, StateMachineBuilder, StateTransition};

// Re-export TurnstileState from protocol crate (single source of truth)
pub use turnkey_protocol::commands::turnstile::TurnstileState;
