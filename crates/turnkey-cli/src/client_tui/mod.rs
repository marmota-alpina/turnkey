//! Client emulator TUI module.
//!
//! Provides a Terminal User Interface for the validation server (client-emulator),
//! allowing operators to manually validate access requests from connected turnstiles.
//!
//! # Architecture
//!
//! The TUI consists of these components:
//!
//! - `app`: Main app state and event loop coordination
//! - `connected_devices`: Panel showing all connected turnstiles
//! - `request_panel`: Display for current access request
//! - `decision_panel`: Buttons and inputs for operator decision (G/E/B/D)
//! - `history`: Scrollable log of validation decisions
//! - `theme`: Color themes and styling (reused from turnstile TUI)

pub mod app;
pub mod connected_devices;
pub mod decision_panel;
pub mod history;
pub mod request_panel;
pub mod theme;

#[cfg(test)]
mod tests;

pub use app::run_client_tui;
