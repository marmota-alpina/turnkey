//! Turnkey emulator crate providing device emulation functionality.
//!
//! This crate contains the state machine and logic for emulating
//! physical access control devices like turnstiles.

pub mod state_machine;

pub use state_machine::{StateMachine, StateMachineBuilder, StateTransition, TurnstileState};
