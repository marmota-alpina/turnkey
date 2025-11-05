//! Terminal User Interface (TUI) module.
//!
//! Provides a complete ratatui-based TUI for the turnstile emulator,
//! mimicking the appearance and behavior of real Brazilian access control
//! devices (Primme SF, Henry Lumen).
//!
//! # Architecture
//!
//! The TUI consists of these components:
//!
//! - `app`: Main app state and event loop coordination
//! - `display`: LCD display rendering (2 lines x 40 columns)
//! - `keypad`: Virtual keypad rendering (4x3 grid + action buttons)
//! - `logs`: Scrollable event log panel
//! - `status`: Status bar with reader and system information
//! - `theme`: Color themes and styling

pub mod app;
pub mod display;
pub mod keypad;
pub mod logs;
pub mod status;
pub mod theme;

pub use app::run_tui;
