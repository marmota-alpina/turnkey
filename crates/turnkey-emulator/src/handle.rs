//! External observability handles for the turnstile emulator.
//!
//! This module provides types and channels for external systems (like TUI)
//! to observe emulator state changes and control emulator behavior without
//! blocking the main event loop.
//!
//! # Architecture
//!
//! The handle-based architecture uses Tokio channels for reactive updates:
//!
//! ```text
//! TurnstileEmulator (background task)
//!     |
//!     ├─> watch::Sender<TurnstileState> ─┐
//!     ├─> watch::Sender<DisplayState> ───┤
//!     ├─> broadcast::Sender<EmulatorEvent> ┐
//!     └─< mpsc::Receiver<InputCommand> ───┘
//!                                           │
//!                                           v
//!                                   EmulatorHandle
//!                                           │
//!                                           v
//!                                    TUI / Observer
//! ```
//!
//! # Channel Types
//!
//! - **watch**: For state snapshots (always provides latest value)
//! - **broadcast**: For event streams (ring buffer for multiple subscribers)
//! - **mpsc**: For input commands (single producer, single consumer)
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use turnkey_emulator::TurnstileEmulator;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     # let peripheral_handle = unimplemented!();
//!     # let validator = unimplemented!();
//!     # let config = unimplemented!();
//!     # let device_id = unimplemented!();
//!     let emulator = TurnstileEmulator::new(
//!         peripheral_handle,
//!         validator,
//!         config,
//!         device_id,
//!     )?;
//!
//!     let (handle, task) = emulator.spawn();
//!
//!     // Spawn task to observe events
//!     tokio::spawn(async move {
//!         let mut event_rx = handle.subscribe_events();
//!         while let Ok(event) = event_rx.recv().await {
//!             println!("Event: {:?}", event);
//!         }
//!     });
//!
//!     // Wait for emulator to complete
//!     task.await??;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## State Observation
//!
//! ```no_run
//! # use turnkey_emulator::EmulatorHandle;
//! # async fn example(handle: EmulatorHandle) {
//! let mut state_rx = handle.state_receiver();
//! while state_rx.changed().await.is_ok() {
//!     let state = *state_rx.borrow();
//!     println!("State changed to: {:?}", state);
//! }
//! # }
//! ```
//!
//! ## Send Input Commands
//!
//! ```no_run
//! # use turnkey_emulator::{EmulatorHandle, InputCommand};
//! # async fn example(handle: EmulatorHandle) -> Result<(), Box<dyn std::error::Error>> {
//! handle.send_input(InputCommand::KeypadDigit(1)).await?;
//! handle.send_input(InputCommand::KeypadDigit(2)).await?;
//! handle.send_input(InputCommand::KeypadEnter).await?;
//! # Ok(())
//! # }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, watch};

use turnkey_core::{ReaderType, Result};
use turnkey_protocol::commands::access::AccessResponse;

use crate::{OperationMode, TurnstileState};

/// Channel capacity for event broadcast channel.
///
/// This capacity allows up to 100 events to be buffered for slow consumers.
/// Exceeding this capacity will cause oldest events to be dropped (lagged).
///
/// # Event Rate Characteristics
///
/// Typical event rates during normal operation:
/// - **Idle state**: 0-1 events/second (only periodic status updates)
/// - **Active access**: 10-50 events/second (keypad input, validation, state transitions)
/// - **High throughput**: 100+ events/second (multiple simultaneous accesses)
///
/// # Tuning Guidance
///
/// **Increase this value if**:
/// - Multiple slow observers are expected (e.g., logging + TUI + metrics)
/// - High-frequency events cause lag warnings in production logs
/// - Running on systems with high concurrent access rates
///
/// **Decrease this value if**:
/// - Memory usage is a concern in embedded/constrained deployments
/// - Fast observer response is required (fail-fast instead of buffering)
/// - Single observer with guaranteed processing speed
///
/// # Performance Impact
///
/// - Memory: ~8KB per 100 events (assuming ~80 bytes per event average)
/// - CPU: Minimal (ring buffer with atomic operations)
/// - Latency: Sub-microsecond for send, zero-copy for receive
pub const EVENT_CHANNEL_CAPACITY: usize = 100;

/// Channel capacity for input command channel.
///
/// This capacity allows up to 32 input commands to be queued.
///
/// # Input Rate Characteristics
///
/// Typical input rates:
/// - **Human keypad input**: 2-5 commands/second (typing speed)
/// - **Automated testing**: 10-100 commands/second (scripted input)
/// - **Burst scenarios**: Up to 32 commands can queue before backpressure
///
/// # Tuning Guidance
///
/// This value is generally sufficient for all use cases. Consider adjustment only if:
/// - Running automated high-speed testing (increase to 64-128)
/// - Extremely memory-constrained environment (decrease to 16)
pub const INPUT_CHANNEL_CAPACITY: usize = 32;

/// Handle for observing and controlling a running turnstile emulator.
///
/// This handle provides access to emulator state, display content, and events
/// without blocking the main event loop. It enables external systems like TUI
/// to observe emulator behavior in real-time.
///
/// # Cloning
///
/// `EmulatorHandle` can be cloned to create multiple observers. Each clone
/// receives independent event streams but shares the same state receivers.
///
/// # Examples
///
/// ```no_run
/// # use turnkey_emulator::TurnstileEmulator;
/// # async fn example(emulator: TurnstileEmulator) -> Result<(), Box<dyn std::error::Error>> {
/// let (handle, task) = emulator.spawn();
///
/// // Clone handle for multiple observers
/// let handle2 = handle.clone();
///
/// // First observer
/// tokio::spawn(async move {
///     let mut events = handle.subscribe_events();
///     while let Ok(event) = events.recv().await {
///         println!("Observer 1: {:?}", event);
///     }
/// });
///
/// // Second observer
/// tokio::spawn(async move {
///     let mut events = handle2.subscribe_events();
///     while let Ok(event) = events.recv().await {
///         println!("Observer 2: {:?}", event);
///     }
/// });
///
/// task.await??;
/// # Ok(())
/// # }
/// ```
pub struct EmulatorHandle {
    /// Watch channel for state machine updates.
    state_rx: watch::Receiver<TurnstileState>,

    /// Watch channel for display content updates.
    display_rx: watch::Receiver<DisplayState>,

    /// Broadcast channel for emulator events.
    event_tx: broadcast::Sender<EmulatorEvent>,

    /// Command channel to send input to emulator.
    input_tx: mpsc::Sender<InputCommand>,
}

impl EmulatorHandle {
    /// Create a new emulator handle with the provided channel receivers.
    ///
    /// This method is internal and should only be called by `TurnstileEmulator::spawn()`.
    pub(crate) fn new(
        state_rx: watch::Receiver<TurnstileState>,
        display_rx: watch::Receiver<DisplayState>,
        event_tx: broadcast::Sender<EmulatorEvent>,
        input_tx: mpsc::Sender<InputCommand>,
    ) -> Self {
        Self {
            state_rx,
            display_rx,
            event_tx,
            input_tx,
        }
    }

    /// Get a receiver for state machine updates.
    ///
    /// The returned receiver provides access to the current state and notifies
    /// when the state changes. Use `watch::Receiver::changed()` to wait for
    /// state transitions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::EmulatorHandle;
    /// # async fn example(handle: EmulatorHandle) {
    /// let mut state_rx = handle.state_receiver();
    ///
    /// // Get current state without waiting
    /// let current = *state_rx.borrow();
    /// println!("Current state: {:?}", current);
    ///
    /// // Wait for state change
    /// while state_rx.changed().await.is_ok() {
    ///     let new_state = *state_rx.borrow();
    ///     println!("State changed to: {:?}", new_state);
    /// }
    /// # }
    /// ```
    pub fn state_receiver(&self) -> watch::Receiver<TurnstileState> {
        self.state_rx.clone()
    }

    /// Get the current state without waiting.
    ///
    /// This is a convenience method that returns the current state value
    /// without requiring async context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::EmulatorHandle;
    /// # fn example(handle: EmulatorHandle) {
    /// let state = handle.current_state();
    /// println!("Current state: {:?}", state);
    /// # }
    /// ```
    pub fn current_state(&self) -> TurnstileState {
        *self.state_rx.borrow()
    }

    /// Get a receiver for display content updates.
    ///
    /// The returned receiver provides access to the current display state
    /// and notifies when the display content changes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::EmulatorHandle;
    /// # async fn example(handle: EmulatorHandle) {
    /// let mut display_rx = handle.display_receiver();
    ///
    /// while display_rx.changed().await.is_ok() {
    ///     let display = display_rx.borrow();
    ///     println!("Line 1: {}", display.line1);
    ///     println!("Line 2: {}", display.line2);
    /// }
    /// # }
    /// ```
    pub fn display_receiver(&self) -> watch::Receiver<DisplayState> {
        self.display_rx.clone()
    }

    /// Get the current display state without waiting.
    ///
    /// This is a convenience method that returns the current display state
    /// without requiring async context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::EmulatorHandle;
    /// # fn example(handle: EmulatorHandle) {
    /// let display = handle.current_display();
    /// println!("Display: {} | {}", display.line1, display.line2);
    /// # }
    /// ```
    pub fn current_display(&self) -> DisplayState {
        self.display_rx.borrow().clone()
    }

    /// Subscribe to emulator events.
    ///
    /// Creates a new event receiver that will receive all future events.
    /// Each call creates an independent receiver, allowing multiple observers.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::EmulatorHandle;
    /// # async fn example(handle: EmulatorHandle) {
    /// let mut events = handle.subscribe_events();
    ///
    /// while let Ok(event) = events.recv().await {
    ///     match event {
    ///         turnkey_emulator::EmulatorEvent::KeypadInput(c) => {
    ///             println!("Keypad: {}", c);
    ///         }
    ///         turnkey_emulator::EmulatorEvent::CardRead(card) => {
    ///             println!("Card: {}", card);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # }
    /// ```
    pub fn subscribe_events(&self) -> broadcast::Receiver<EmulatorEvent> {
        self.event_tx.subscribe()
    }

    /// Send an input command to the emulator.
    ///
    /// This allows external systems to control the emulator by simulating
    /// keypad input, card reads, or triggering shutdown.
    ///
    /// # Errors
    ///
    /// Returns error if the emulator task has terminated and can no longer
    /// receive commands.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::{EmulatorHandle, InputCommand};
    /// # async fn example(handle: EmulatorHandle) -> Result<(), Box<dyn std::error::Error>> {
    /// // Simulate keypad entry of "1234"
    /// handle.send_input(InputCommand::KeypadDigit(1)).await?;
    /// handle.send_input(InputCommand::KeypadDigit(2)).await?;
    /// handle.send_input(InputCommand::KeypadDigit(3)).await?;
    /// handle.send_input(InputCommand::KeypadDigit(4)).await?;
    /// handle.send_input(InputCommand::KeypadEnter).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_input(&self, command: InputCommand) -> Result<()> {
        self.input_tx
            .send(command)
            .await
            .map_err(|_| turnkey_core::Error::Config("Emulator task terminated".to_string()))?;
        Ok(())
    }

    /// Send shutdown signal to the emulator.
    ///
    /// This is a convenience method that sends the `InputCommand::Shutdown`
    /// command to gracefully stop the emulator event loop.
    ///
    /// # Errors
    ///
    /// Returns error if the emulator task has already terminated.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::EmulatorHandle;
    /// # async fn example(handle: EmulatorHandle) -> Result<(), Box<dyn std::error::Error>> {
    /// // Request graceful shutdown
    /// handle.shutdown().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn shutdown(&self) -> Result<()> {
        self.send_input(InputCommand::Shutdown).await
    }
}

impl Clone for EmulatorHandle {
    fn clone(&self) -> Self {
        Self {
            state_rx: self.state_rx.clone(),
            display_rx: self.display_rx.clone(),
            event_tx: self.event_tx.clone(),
            input_tx: self.input_tx.clone(),
        }
    }
}

/// Snapshot of the virtual display state.
///
/// Contains the current content of both display lines plus contextual
/// information about the emulator mode and device identity.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::{DisplayState, OperationMode};
///
/// let display = DisplayState {
///     line1: "DIGITE SEU CODIGO".to_string(),
///     line2: "".to_string(),
///     mode: OperationMode::Online,
///     device_id: 15,
///     ip_address: Some("192.168.0.100".to_string()),
/// };
///
/// assert_eq!(display.line1, "DIGITE SEU CODIGO");
/// assert_eq!(display.mode, OperationMode::Online);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayState {
    /// Content of display line 1 (top line).
    pub line1: String,

    /// Content of display line 2 (bottom line).
    pub line2: String,

    /// Current operation mode (Online or Offline).
    pub mode: OperationMode,

    /// Device ID (1-99).
    pub device_id: u8,

    /// IP address for Online mode, if available.
    pub ip_address: Option<String>,
}

impl DisplayState {
    /// Create a new display state.
    pub fn new(
        line1: String,
        line2: String,
        mode: OperationMode,
        device_id: u8,
        ip_address: Option<String>,
    ) -> Self {
        Self {
            line1,
            line2,
            mode,
            device_id,
            ip_address,
        }
    }
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            line1: String::new(),
            line2: String::new(),
            mode: OperationMode::Online,
            device_id: 1,
            ip_address: None,
        }
    }
}

/// Category of error that occurred during emulator operation.
///
/// This enum provides semantic categorization of errors to help
/// observers respond appropriately. For example, a TUI might show
/// different icons or colors for hardware vs network errors.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::ErrorCategory;
///
/// let category = ErrorCategory::Hardware;
/// match category {
///     ErrorCategory::Hardware => println!("Check device connections"),
///     ErrorCategory::Network => println!("Check network connectivity"),
///     _ => println!("General error"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Peripheral device error (USB disconnect, read failure, timeout).
    ///
    /// Indicates hardware communication issues with RFID readers,
    /// biometric scanners, or turnstile controllers.
    Hardware,

    /// Network or validation error (connection lost, timeout, server unavailable).
    ///
    /// Indicates issues with ONLINE mode validation, TCP connectivity,
    /// or server communication.
    Network,

    /// Protocol parsing error (malformed message, invalid checksum, unexpected format).
    ///
    /// Indicates issues parsing or building Henry protocol messages.
    Protocol,

    /// Configuration error (invalid parameters, missing required fields).
    ///
    /// Indicates problems with emulator or device configuration.
    Configuration,

    /// Internal logic error (state machine violation, unexpected condition).
    ///
    /// Indicates bugs or unexpected states in emulator logic.
    /// These should be reported as issues.
    Internal,

    /// Validation error (card not found, access denied, expired credential).
    ///
    /// Indicates expected validation failures during access control.
    Validation,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Hardware => write!(f, "Hardware"),
            Self::Network => write!(f, "Network"),
            Self::Protocol => write!(f, "Protocol"),
            Self::Configuration => write!(f, "Configuration"),
            Self::Internal => write!(f, "Internal"),
            Self::Validation => write!(f, "Validation"),
        }
    }
}

/// Events emitted by the turnstile emulator.
///
/// These events represent all observable actions and state changes
/// in the emulator, allowing external systems to react to emulator
/// behavior in real-time.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::EmulatorEvent;
/// use turnkey_core::ReaderType;
/// use turnkey_protocol::commands::turnstile::TurnstileState;
///
/// let event = EmulatorEvent::StateTransition {
///     from: TurnstileState::Idle,
///     to: TurnstileState::Validating,
/// };
///
/// match event {
///     EmulatorEvent::StateTransition { from, to } => {
///         println!("State changed from {:?} to {:?}", from, to);
///     }
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmulatorEvent {
    /// Keypad digit or symbol entered.
    ///
    /// This event is emitted for each keypad button press (0-9, *, #).
    KeypadInput(char),

    /// RFID/NFC card was read.
    ///
    /// Contains the card number as a string.
    CardRead(String),

    /// Biometric fingerprint was captured.
    ///
    /// Contains the quality score (0-100) of the capture.
    BiometricCapture { quality: u8 },

    /// State machine transitioned to a new state.
    ///
    /// Emitted on every state transition, providing both the previous
    /// and new states.
    StateTransition {
        from: TurnstileState,
        to: TurnstileState,
    },

    /// Protocol message was sent to server (ONLINE mode only).
    ///
    /// Contains the command code and associated state for status messages
    /// (000+80, 000+81, 000+82). This allows observers to track protocol-level
    /// communication with the validation server.
    ProtocolMessageSent {
        command_code: String,
        state: TurnstileState,
    },

    /// Protocol message was received from server (ONLINE mode only).
    ///
    /// Contains the raw protocol message string.
    ProtocolMessageReceived(String),

    /// Validation request initiated.
    ///
    /// Emitted when the emulator starts validating a credential.
    ValidationRequest {
        credential: String,
        reader_type: ReaderType,
    },

    /// Validation result received.
    ///
    /// Emitted when validation completes (grant or deny).
    ValidationResult(AccessResponse),

    /// Error occurred during emulator operation.
    ///
    /// Errors are categorized by type and marked as recoverable or fatal.
    /// Observers can use the category to provide appropriate user feedback
    /// (e.g., show hardware icon for device errors, network icon for connection issues).
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::{EmulatorEvent, ErrorCategory};
    ///
    /// let event = EmulatorEvent::Error {
    ///     message: "USB device disconnected".to_string(),
    ///     category: ErrorCategory::Hardware,
    ///     recoverable: true,
    /// };
    ///
    /// match event {
    ///     EmulatorEvent::Error { category: ErrorCategory::Hardware, .. } => {
    ///         println!("Hardware issue detected");
    ///     }
    ///     _ => {}
    /// }
    /// ```
    Error {
        message: String,
        category: ErrorCategory,
        recoverable: bool,
    },

    /// Display content changed.
    ///
    /// Emitted whenever either display line is updated.
    DisplayUpdated { line1: String, line2: String },

    /// Keypad buffer changed.
    ///
    /// Emitted when keypad digits are added or buffer is cleared.
    KeypadBufferChanged(String),
}

impl fmt::Display for EmulatorEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::KeypadInput(c) => write!(f, "KeypadInput('{}')", c),
            Self::CardRead(card) => write!(f, "CardRead({})", card),
            Self::BiometricCapture { quality } => {
                write!(f, "BiometricCapture(quality={})", quality)
            }
            Self::StateTransition { from, to } => {
                write!(f, "StateTransition({:?} -> {:?})", from, to)
            }
            Self::ProtocolMessageSent {
                command_code,
                state,
            } => write!(
                f,
                "ProtocolMessageSent({}, state: {:?})",
                command_code, state
            ),
            Self::ProtocolMessageReceived(msg) => write!(f, "ProtocolMessageReceived({})", msg),
            Self::ValidationRequest {
                credential,
                reader_type,
            } => write!(f, "ValidationRequest({}, {:?})", credential, reader_type),
            Self::ValidationResult(resp) => write!(f, "ValidationResult({:?})", resp),
            Self::Error {
                message,
                category,
                recoverable,
            } => {
                if *recoverable {
                    write!(f, "Error({}, recoverable: {})", category, message)
                } else {
                    write!(f, "Error({}, fatal: {})", category, message)
                }
            }
            Self::DisplayUpdated { line1, line2 } => {
                write!(f, "DisplayUpdated('{}' | '{}')", line1, line2)
            }
            Self::KeypadBufferChanged(buffer) => write!(f, "KeypadBufferChanged('{}')", buffer),
        }
    }
}

/// Commands that can be sent to control the emulator externally.
///
/// These commands allow external systems (like TUI) to simulate peripheral
/// input or control emulator lifecycle without direct hardware access.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::InputCommand;
///
/// let commands = vec![
///     InputCommand::KeypadDigit(1),
///     InputCommand::KeypadDigit(2),
///     InputCommand::KeypadDigit(3),
///     InputCommand::KeypadDigit(4),
///     InputCommand::KeypadEnter,
/// ];
///
/// // These commands would simulate entering "1234" on the keypad
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputCommand {
    /// Simulate keypad digit press (0-9).
    KeypadDigit(u8),

    /// Simulate ENTER key press.
    KeypadEnter,

    /// Simulate CANCEL key press.
    KeypadCancel,

    /// Simulate CLEAR key press (backspace).
    KeypadClear,

    /// Simulate RFID/NFC card read.
    ///
    /// Contains the card number to simulate.
    SimulateCardRead(String),

    /// Simulate biometric fingerprint capture.
    ///
    /// Contains the quality score (0-100) and optional user ID
    /// for successful matches.
    SimulateBiometric { quality: u8, user_id: Option<u32> },

    /// Request graceful shutdown of the emulator.
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_state_default() {
        let display = DisplayState::default();
        assert_eq!(display.line1, "");
        assert_eq!(display.line2, "");
        assert_eq!(display.mode, OperationMode::Online);
        assert_eq!(display.device_id, 1);
        assert_eq!(display.ip_address, None);
    }

    #[test]
    fn test_display_state_new() {
        let display = DisplayState::new(
            "Line 1".to_string(),
            "Line 2".to_string(),
            OperationMode::Offline,
            15,
            Some("192.168.1.1".to_string()),
        );

        assert_eq!(display.line1, "Line 1");
        assert_eq!(display.line2, "Line 2");
        assert_eq!(display.mode, OperationMode::Offline);
        assert_eq!(display.device_id, 15);
        assert_eq!(display.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_emulator_event_display() {
        let event = EmulatorEvent::KeypadInput('5');
        assert_eq!(event.to_string(), "KeypadInput('5')");

        let event = EmulatorEvent::CardRead("1234567890".to_string());
        assert_eq!(event.to_string(), "CardRead(1234567890)");

        let event = EmulatorEvent::StateTransition {
            from: TurnstileState::Idle,
            to: TurnstileState::Validating,
        };
        assert_eq!(event.to_string(), "StateTransition(Idle -> Validating)");
    }

    #[test]
    fn test_input_command_equality() {
        assert_eq!(InputCommand::KeypadDigit(5), InputCommand::KeypadDigit(5));
        assert_ne!(InputCommand::KeypadDigit(5), InputCommand::KeypadDigit(6));
        assert_eq!(InputCommand::KeypadEnter, InputCommand::KeypadEnter);
        assert_ne!(InputCommand::KeypadEnter, InputCommand::KeypadCancel);
    }
}
