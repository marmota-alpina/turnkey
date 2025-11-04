//! Turnstile emulator core logic.
//!
//! This module provides the main `TurnstileEmulator` struct that integrates
//! all components: peripherals, state machine, validator, and display.
//!
//! # Architecture
//!
//! ```text
//! TurnstileEmulator
//! ├── PeripheralHandle → receives input events
//! ├── StateMachine → tracks turnstile state
//! ├── Validator → validates access requests
//! ├── VirtualDisplay → shows messages to user
//! └── EmulatorConfig → configuration
//! ```
//!
//! # Event Loop
//!
//! The emulator runs an async event loop using `tokio::select!`:
//! - Receives peripheral events (keypad, RFID, biometric)
//! - Handles state machine timeouts
//! - Processes validation responses
//! - Updates display
//!
//! # Examples
//!
//! ```ignore
//! use turnkey_emulator::{TurnstileEmulator, EmulatorConfig, OperationMode};
//! use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
//! use turnkey_storage::{Validator, OfflineValidator};
//! use turnkey_core::DeviceId;
//!
//! // Setup peripherals
//! let peripheral_config = PeripheralConfig::default();
//! let mut peripheral_manager = PeripheralManager::new(peripheral_config);
//! // ... register devices ...
//! let peripheral_handle = peripheral_manager.start();
//!
//! // Setup validator
//! let validator = Validator::Offline(OfflineValidator::new(pool));
//!
//! // Create emulator
//! let config = EmulatorConfig::default();
//! let device_id = DeviceId::new(config.device_id)?;
//! let emulator = TurnstileEmulator::new(peripheral_handle, validator, config, device_id)?;
//!
//! // Run emulator
//! emulator.run().await?;
//! ```

use std::time::{Duration, Instant};

use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, error, info, trace, warn};

use turnkey_core::{DeviceId, Error, HenryTimestamp, ReaderType, Result};
use turnkey_hardware::manager::{PeripheralEvent, PeripheralHandle};
use turnkey_hardware::{BiometricData, KeypadInput};
use turnkey_protocol::commands::access::{AccessRequest, AccessResponse};
use turnkey_storage::Validator;

use crate::TurnstileState;
use crate::config::{EmulatorConfig, OperationMode};
use crate::display::VirtualDisplay;
use crate::handle::{
    DisplayState, EVENT_CHANNEL_CAPACITY, EmulatorEvent, EmulatorHandle, ErrorCategory,
    INPUT_CHANNEL_CAPACITY, InputCommand,
};
use crate::state_machine::StateMachine;

/// Internal helper for broadcasting events and managing state updates.
///
/// This struct encapsulates the channel senders to reduce parameter repetition
/// and provide a clean interface for event broadcasting (Single Responsibility Principle).
struct BroadcastChannels {
    state_tx: watch::Sender<TurnstileState>,
    display_tx: watch::Sender<DisplayState>,
    event_tx: broadcast::Sender<EmulatorEvent>,
}

impl BroadcastChannels {
    /// Broadcast an error event with proper categorization.
    ///
    /// This centralizes error broadcasting logic to avoid duplication.
    fn broadcast_error(&self, message: String, category: ErrorCategory, recoverable: bool) {
        let _ = self.event_tx.send(EmulatorEvent::Error {
            message,
            category,
            recoverable,
        });
    }

    /// Broadcast a state transition.
    fn broadcast_state_transition(&self, from: TurnstileState, to: TurnstileState) {
        let _ = self.state_tx.send(to);
        let _ = self
            .event_tx
            .send(EmulatorEvent::StateTransition { from, to });
    }

    /// Broadcast a display update.
    fn broadcast_display(&self, display: DisplayState) {
        let line1 = display.line1.clone();
        let line2 = display.line2.clone();
        let _ = self.display_tx.send(display);
        let _ = self
            .event_tx
            .send(EmulatorEvent::DisplayUpdated { line1, line2 });
    }

    /// Broadcast a generic event.
    fn broadcast_event(&self, event: EmulatorEvent) {
        let _ = self.event_tx.send(event);
    }
}

/// Minimum credential length per Henry protocol specification.
const MIN_CREDENTIAL_LENGTH: usize = 3;

/// Maximum credential length per Henry protocol specification.
const MAX_CREDENTIAL_LENGTH: usize = 20;

/// Maximum keypad buffer length (same as max credential length).
const MAX_BUFFER_LENGTH: usize = MAX_CREDENTIAL_LENGTH;

/// Error display duration in seconds.
const ERROR_DISPLAY_DURATION_SECS: u64 = 2;

/// Access denied display duration in seconds.
///
/// Time to show "Access denied" message before returning to idle.
/// Must be long enough for user to read message but not block throughput.
const ACCESS_DENIED_DISPLAY_DURATION_SECS: u64 = 3;

/// Timeout recovery delay in seconds.
///
/// Pause after timeout state before returning to idle, allowing user
/// to see timeout message and preventing rapid state cycling.
const TIMEOUT_RECOVERY_DELAY_SECS: u64 = 2;

/// Post-rotation completion delay in milliseconds.
///
/// Brief stabilization period after rotation completes before returning
/// to idle. Allows mechanical systems to settle and prevents bouncing.
const POST_ROTATION_DELAY_MS: u64 = 500;

/// Main turnstile emulator struct.
///
/// Orchestrates all emulator components and handles the main event loop.
///
/// # Lifecycle
///
/// 1. Create with `new()` or `builder()`
/// 2. Call `run()` to start event loop
/// 3. Event loop runs until error or shutdown signal
/// 4. Graceful cleanup on shutdown
///
/// # Examples
///
/// ```ignore
/// // Create emulator
/// let emulator = TurnstileEmulator::new(
///     peripheral_handle,
///     validator,
///     config,
///     device_id,
/// )?;
///
/// // Run until shutdown
/// emulator.run().await?;
/// ```
pub struct TurnstileEmulator {
    /// Handle for receiving peripheral events.
    peripheral_handle: PeripheralHandle,

    /// State machine for tracking turnstile state.
    state_machine: StateMachine,

    /// Virtual LCD display for user messages.
    display: VirtualDisplay,

    /// Validator for access requests (Online or Offline).
    validator: Validator,

    /// Device ID for protocol messages.
    device_id: DeviceId,

    /// Emulator configuration.
    config: EmulatorConfig,

    /// Keypad input buffer.
    keypad_buffer: String,

    /// Last activity timestamp (for idle detection).
    last_activity: Instant,

    /// Last credential used for access (needed for protocol rotation messages).
    last_credential: Option<String>,
}

impl TurnstileEmulator {
    /// Create a new turnstile emulator.
    ///
    /// # Arguments
    ///
    /// * `peripheral_handle` - Handle for receiving peripheral events
    /// * `validator` - Validator for access requests (Online or Offline)
    /// * `config` - Emulator configuration
    /// * `device_id` - Device ID for protocol messages
    ///
    /// # Returns
    ///
    /// Returns `Ok(TurnstileEmulator)` if all components were successfully
    /// initialized.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Display initialization fails
    /// - State machine initialization fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::{TurnstileEmulator, EmulatorConfig};
    /// # use turnkey_core::DeviceId;
    /// # async fn example(
    /// #     peripheral_handle: turnkey_hardware::manager::PeripheralHandle,
    /// #     validator: turnkey_storage::Validator,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// let config = EmulatorConfig::default();
    /// let device_id = DeviceId::new(1)?;
    ///
    /// let emulator = TurnstileEmulator::new(
    ///     peripheral_handle,
    ///     validator,
    ///     config,
    ///     device_id,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        peripheral_handle: PeripheralHandle,
        validator: Validator,
        config: EmulatorConfig,
        device_id: DeviceId,
    ) -> Result<Self> {
        let display = VirtualDisplay::new(2, 40, config.default_display_message.clone());
        let state_machine = StateMachine::new();

        Ok(Self {
            peripheral_handle,
            state_machine,
            display,
            validator,
            device_id,
            config,
            keypad_buffer: String::new(),
            last_activity: Instant::now(),
            last_credential: None,
        })
    }

    /// Spawn the emulator in a background task with observability handles.
    ///
    /// This method creates channel infrastructure for external observers
    /// and spawns the emulator event loop as a Tokio task. The returned
    /// handle allows observers to receive state updates, display changes,
    /// and events without blocking the emulator.
    ///
    /// # Returns
    ///
    /// Returns a tuple of:
    /// - `EmulatorHandle` - Handle for observing and controlling the emulator
    /// - `JoinHandle<Result<()>>` - Task handle for the background event loop
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::TurnstileEmulator;
    /// # async fn example(emulator: TurnstileEmulator) -> Result<(), Box<dyn std::error::Error>> {
    /// let (handle, task) = emulator.spawn();
    ///
    /// // Spawn observer task
    /// tokio::spawn(async move {
    ///     let mut events = handle.subscribe_events();
    ///     while let Ok(event) = events.recv().await {
    ///         println!("Event: {:?}", event);
    ///     }
    /// });
    ///
    /// // Wait for emulator to complete
    /// task.await??;
    /// # Ok(())
    /// # }
    /// ```
    pub fn spawn(self) -> (EmulatorHandle, JoinHandle<Result<()>>) {
        // Create channels for state broadcasting
        let (state_tx, state_rx) = watch::channel(TurnstileState::Idle);

        // Determine IP address for display state
        let ip_address = if matches!(self.config.mode, OperationMode::Online) {
            self.config.network.as_ref().map(|n| n.ip_address_string())
        } else {
            None
        };

        let (display_tx, display_rx) = watch::channel(DisplayState {
            mode: self.config.mode,
            device_id: self.device_id.as_u8(),
            line1: self.config.default_display_message.clone(),
            line2: String::new(),
            ip_address,
        });
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let (input_tx, input_rx) = mpsc::channel(INPUT_CHANNEL_CAPACITY);

        // Create handle for observers
        let handle = EmulatorHandle::new(state_rx, display_rx, event_tx.clone(), input_tx);

        // Spawn background task
        let task = tokio::spawn(async move {
            self.run_with_channels(state_tx, display_tx, event_tx, input_rx)
                .await
        });

        (handle, task)
    }

    /// Run the emulator event loop.
    ///
    /// This method provides backward compatibility and convenience for users
    /// who don't need external observability. Internally it uses `spawn()`
    /// and waits for the background task to complete.
    ///
    /// For external observability, use `spawn()` instead.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on graceful shutdown.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - A fatal error occurs during event processing
    /// - State machine enters invalid state
    /// - Display update fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_emulator::TurnstileEmulator;
    /// # async fn example(emulator: TurnstileEmulator) -> Result<(), Box<dyn std::error::Error>> {
    /// emulator.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(self) -> Result<()> {
        let (_handle, task) = self.spawn();
        task.await
            .map_err(|e| turnkey_core::Error::Config(format!("Task panicked: {}", e)))?
    }

    /// Internal event loop with channel broadcasting.
    ///
    /// This method is called by `spawn()` and runs the main event loop
    /// while broadcasting state changes and events to observers.
    async fn run_with_channels(
        mut self,
        state_tx: watch::Sender<TurnstileState>,
        display_tx: watch::Sender<DisplayState>,
        event_tx: broadcast::Sender<EmulatorEvent>,
        mut input_rx: mpsc::Receiver<InputCommand>,
    ) -> Result<()> {
        info!(
            device_id = %self.device_id,
            mode = ?self.config.mode,
            "Starting turnstile emulator with observability"
        );

        // Create broadcast channels helper to eliminate parameter repetition
        let channels = BroadcastChannels {
            state_tx,
            display_tx,
            event_tx,
        };

        loop {
            // Get timeout duration before select! to avoid borrow issues
            let timeout_duration = self.get_state_timeout_duration();
            let should_timeout = self.should_check_timeout();
            let idle_timeout = self.get_idle_timeout_duration();
            let should_check_idle = self.config.idle_timeout_secs > 0;

            tokio::select! {
                // Peripheral events
                Some(event) = self.peripheral_handle.recv() => {
                    if let Err(e) = self.handle_peripheral_event(event, &channels).await {
                        error!(error = %e, "Error handling peripheral event");
                        channels.broadcast_error(
                            e.to_string(),
                            ErrorCategory::Hardware,
                            true
                        );
                        // Continue running - peripheral errors are recoverable
                    }
                }

                // Input commands from external observers
                Some(cmd) = input_rx.recv() => {
                    match self.handle_input_command(cmd, &channels).await {
                        Ok(should_shutdown) => {
                            if should_shutdown {
                                info!("Shutdown requested via input command");
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Error handling input command");
                            channels.broadcast_error(
                                e.to_string(),
                                ErrorCategory::Configuration,
                                true
                            );
                        }
                    }
                }

                // State timeouts
                _ = sleep(timeout_duration), if should_timeout => {
                    if let Err(e) = self.handle_state_timeout(&channels).await {
                        error!(error = %e, "Error handling state timeout");
                        channels.broadcast_error(
                            e.to_string(),
                            ErrorCategory::Internal,
                            true
                        );
                        // Try to recover to Idle state
                        let _ = self.return_to_idle_with_channels(&channels);
                    }
                }

                // Idle timeout
                _ = sleep(idle_timeout), if should_check_idle => {
                    if let Err(e) = self.handle_idle_timeout(&channels) {
                        error!(error = %e, "Error handling idle timeout");
                        channels.broadcast_error(
                            e.to_string(),
                            ErrorCategory::Internal,
                            true
                        );
                        let _ = self.return_to_idle_with_channels(&channels);
                    }
                }
            }
        }
    }

    /// Handle peripheral event.
    ///
    /// Dispatches the event to the appropriate handler based on event type.
    async fn handle_peripheral_event(
        &mut self,
        event: PeripheralEvent,
        channels: &BroadcastChannels,
    ) -> Result<()> {
        trace!(?event, "Received peripheral event");
        self.last_activity = Instant::now();

        match event {
            PeripheralEvent::KeypadInput(input) => self.handle_keypad_input(input, channels).await,
            PeripheralEvent::CardRead(card) => {
                let credential = card.uid_decimal();
                info!(uid = %credential, "Card read");

                channels.broadcast_event(EmulatorEvent::CardRead(credential.clone()));

                self.process_credential(credential, ReaderType::Rfid, channels)
                    .await
            }
            PeripheralEvent::FingerprintCaptured(bio) => {
                info!(quality = bio.quality, "Fingerprint captured");
                channels.broadcast_event(EmulatorEvent::BiometricCapture {
                    quality: bio.quality,
                });
                self.handle_biometric_capture(bio).await
            }
            PeripheralEvent::DeviceError { device_type, error } => {
                warn!(%device_type, %error, "Device error");
                channels.broadcast_error(
                    format!("Device error: {}: {}", device_type, error),
                    ErrorCategory::Hardware,
                    true,
                );
                Ok(())
            }
            _ => {
                warn!("Unknown peripheral event");
                Ok(())
            }
        }
    }

    /// Handle input command from external observers.
    ///
    /// Returns `true` if shutdown was requested, `false` otherwise.
    async fn handle_input_command(
        &mut self,
        cmd: InputCommand,
        channels: &BroadcastChannels,
    ) -> Result<bool> {
        match cmd {
            InputCommand::KeypadDigit(digit) => {
                if digit > 9 {
                    return Err(turnkey_core::Error::Config(format!(
                        "Invalid digit: {}",
                        digit
                    )));
                }
                self.append_digit(digit, channels);
                Ok(false)
            }
            InputCommand::KeypadEnter => {
                self.process_keypad_entry(channels).await?;
                Ok(false)
            }
            InputCommand::KeypadCancel => {
                self.clear_keypad_buffer(channels);
                Ok(false)
            }
            InputCommand::KeypadClear => {
                self.clear_last_digit(channels);
                Ok(false)
            }
            InputCommand::SimulateCardRead(card) => {
                channels.broadcast_event(EmulatorEvent::CardRead(card.clone()));
                self.process_credential(card, ReaderType::Rfid, channels)
                    .await?;
                Ok(false)
            }
            InputCommand::SimulateBiometric {
                quality,
                user_id: _,
            } => {
                channels.broadcast_event(EmulatorEvent::BiometricCapture { quality });
                // For now, just log - full biometric support not implemented
                warn!("Biometric validation not yet implemented");
                Ok(false)
            }
            InputCommand::Shutdown => {
                info!("Shutdown command received");
                Ok(true)
            }
        }
    }

    /// Handle keypad input.
    async fn handle_keypad_input(
        &mut self,
        input: KeypadInput,
        channels: &BroadcastChannels,
    ) -> Result<()> {
        match input {
            KeypadInput::Digit(digit) => {
                self.append_digit(digit, channels);
                Ok(())
            }
            KeypadInput::Enter => self.process_keypad_entry(channels).await,
            KeypadInput::Cancel => {
                self.clear_keypad_buffer(channels);
                Ok(())
            }
            KeypadInput::Clear => {
                self.clear_last_digit(channels);
                Ok(())
            }
            _ => {
                // Ignore other keypad inputs (Star, Hash, FunctionKey, etc.)
                debug!("Unhandled keypad input: {:?}", input);
                Ok(())
            }
        }
    }

    /// Validate credential length according to Henry protocol.
    ///
    /// Returns `true` if credential length is valid (3-20 characters).
    fn is_valid_credential_length(credential: &str) -> bool {
        (MIN_CREDENTIAL_LENGTH..=MAX_CREDENTIAL_LENGTH).contains(&credential.len())
    }

    /// Validate and process credential from any reader type.
    ///
    /// Checks credential length according to Henry protocol specification
    /// and dispatches to validation flow. Shows appropriate error messages
    /// based on reader type if credential is invalid.
    ///
    /// # Arguments
    ///
    /// * `credential` - The credential string to validate (card number, keypad code, etc.)
    /// * `reader_type` - Type of reader that captured the credential
    /// * `channels` - Broadcast channels for event notification
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if credential was successfully validated and processed.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails or credential processing encounters
    /// an error condition.
    async fn process_credential(
        &mut self,
        credential: String,
        reader_type: ReaderType,
        channels: &BroadcastChannels,
    ) -> Result<()> {
        if !Self::is_valid_credential_length(&credential) {
            warn!(
                credential_length = credential.len(),
                ?reader_type,
                "Invalid credential length (must be {}-{} characters)",
                MIN_CREDENTIAL_LENGTH,
                MAX_CREDENTIAL_LENGTH
            );

            // Error messages based on reader type and credential characteristics
            // Note: Both card and keypad use Rfid reader type in protocol
            let (error_line1, error_line2) = match reader_type {
                ReaderType::Rfid => {
                    // Keypad entries are typically shorter, cards are longer
                    if credential.len() <= 6 {
                        ("CODIGO INVALIDO", "3-20 digitos")
                    } else {
                        ("CARTAO INVALIDO", "Contacte admin")
                    }
                }
                ReaderType::Biometric => ("DIGITAL INVALIDA", "Tente novamente"),
            };

            return self
                .show_error_and_return_to_idle(error_line1, error_line2)
                .await;
        }

        self.validate_credential(credential, reader_type, channels)
            .await
    }

    /// Show error message on display and return to idle after delay.
    async fn show_error_and_return_to_idle(&mut self, line1: &str, line2: &str) -> Result<()> {
        let _ = self.display.set_line(0, line1);
        let _ = self.display.set_line(1, line2);
        sleep(Duration::from_secs(ERROR_DISPLAY_DURATION_SECS)).await;
        self.return_to_idle()
    }

    /// Append digit to keypad buffer.
    fn append_digit(&mut self, digit: u8, channels: &BroadcastChannels) {
        if self.keypad_buffer.len() < MAX_BUFFER_LENGTH {
            // Convert digit (0-9) to char
            let digit_char = (b'0' + digit) as char;
            self.keypad_buffer.push(digit_char);
            debug!(buffer = %self.keypad_buffer, "Digit appended");

            // Broadcast keypad input event
            channels.broadcast_event(EmulatorEvent::KeypadInput(digit_char));

            // Update display to show buffer
            let display_text = format!("Codigo: {}", self.keypad_buffer);
            let _ = self.display.set_line(1, &display_text);

            // Broadcast buffer changed event
            channels.broadcast_event(EmulatorEvent::KeypadBufferChanged(
                self.keypad_buffer.clone(),
            ));
        }
    }

    /// Process keypad entry (ENTER pressed).
    async fn process_keypad_entry(&mut self, channels: &BroadcastChannels) -> Result<()> {
        if self.keypad_buffer.is_empty() {
            debug!("Empty keypad buffer, ignoring ENTER");
            return Ok(());
        }

        let credential = self.keypad_buffer.clone();
        self.keypad_buffer.clear();

        info!(credential = %credential, "Processing keypad entry");
        self.process_credential(credential, ReaderType::Rfid, channels)
            .await
    }

    /// Clear keypad buffer.
    fn clear_keypad_buffer(&mut self, channels: &BroadcastChannels) {
        if !self.keypad_buffer.is_empty() {
            debug!("Clearing keypad buffer");
            self.keypad_buffer.clear();
            let _ = self.display.set_line(1, "");

            // Broadcast buffer changed event
            channels.broadcast_event(EmulatorEvent::KeypadBufferChanged(String::new()));
        }
    }

    /// Clear last digit from keypad buffer.
    fn clear_last_digit(&mut self, channels: &BroadcastChannels) {
        if !self.keypad_buffer.is_empty() {
            self.keypad_buffer.pop();
            debug!(buffer = %self.keypad_buffer, "Last digit cleared");

            let display_text = if self.keypad_buffer.is_empty() {
                String::new()
            } else {
                format!("Codigo: {}", self.keypad_buffer)
            };
            let _ = self.display.set_line(1, &display_text);

            // Broadcast buffer changed event
            channels.broadcast_event(EmulatorEvent::KeypadBufferChanged(
                self.keypad_buffer.clone(),
            ));
        }
    }

    /// Handle biometric capture.
    async fn handle_biometric_capture(&mut self, _bio: BiometricData) -> Result<()> {
        // For now, biometric validation is not implemented
        // In future: extract template and validate
        warn!("Biometric validation not yet implemented");
        Ok(())
    }

    /// Send turnstile status message to server (ONLINE mode only).
    ///
    /// Sends protocol status messages (000+80, 000+81, 000+82) to inform
    /// the server about turnstile rotation state changes. These messages
    /// are critical for proper access logging and server-side state tracking.
    ///
    /// # Protocol Messages
    ///
    /// - `000+80`: Waiting for rotation (user granted, turnstile released)
    /// - `000+81`: Rotation completed (user passed through)
    /// - `000+82`: Rotation timeout (user did not pass in time)
    ///
    /// # Arguments
    ///
    /// * `state` - The turnstile state to report
    /// * `channels` - Broadcast channels for event notification
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if message was sent or mode is OFFLINE.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No credential is available (state tracking error)
    /// - Message broadcasting fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // After granting access and releasing turnstile
    /// self.send_status_message(TurnstileState::WaitingRotation, channels).await?;
    /// ```
    async fn send_status_message(
        &mut self,
        state: TurnstileState,
        channels: &BroadcastChannels,
    ) -> Result<()> {
        // Only send in ONLINE mode
        if !matches!(self.config.mode, OperationMode::Online) {
            return Ok(());
        }

        // Get protocol command code for state
        let command_code = match state.protocol_command_code() {
            Some(code) => code,
            None => {
                // State doesn't emit protocol messages (e.g., Idle, Reading)
                return Ok(());
            }
        };

        // Verify we have a credential to include in status message
        let credential = self.last_credential.as_ref().ok_or_else(|| {
            Error::Config(
                "No credential available for status message - state tracking error".to_string(),
            )
        })?;

        info!(
            %command_code,
            credential_len = credential.len(),
            ?state,
            "Broadcasting turnstile status message"
        );

        // Broadcast protocol message event for observers
        // This allows external systems (like TUI or logging) to see
        // protocol-level communication
        channels.broadcast_event(EmulatorEvent::ProtocolMessageSent {
            command_code: command_code.to_string(),
            state,
        });

        // TODO: Send actual protocol message via validator's network layer
        //
        // Architecture note: This requires adding a send_status() method to
        // the Validator trait, or creating a separate StatusReporter component.
        //
        // Implementation would look like:
        //
        // let timestamp = HenryTimestamp::now();
        // let reader_type = self.last_reader_type.unwrap_or(ReaderType::Rfid);
        //
        // let status = TurnstileStatus::new(
        //     credential.clone(),
        //     timestamp,
        //     self.config.default_direction,
        //     reader_type,
        // )?;
        //
        // self.validator.send_status(status).await?;

        Ok(())
    }

    /// Validate credential (card number or keypad code).
    async fn validate_credential(
        &mut self,
        credential: String,
        reader_type: ReaderType,
        channels: &BroadcastChannels,
    ) -> Result<()> {
        // Transition to Validating state
        let old_state = *self.state_machine.current_state();
        self.state_machine
            .transition_to(TurnstileState::Validating)?;
        self.display.update_from_state(&TurnstileState::Validating);

        // Broadcast state transition
        channels.broadcast_state_transition(old_state, TurnstileState::Validating);

        debug!(%credential, ?reader_type, "Validating credential");

        // Save credential for later use in rotation messages
        self.last_credential = Some(credential.clone());

        // Create access request
        let request = AccessRequest::new(
            credential.clone(),
            HenryTimestamp::now(),
            self.config.default_direction,
            reader_type,
        )?;

        // Broadcast validation request event
        channels.broadcast_event(EmulatorEvent::ValidationRequest {
            credential: credential.clone(),
            reader_type,
        });

        // Validate (synchronous async call - blocks until response)
        let response = match self.validator.validate(&request).await {
            Ok(resp) => resp,
            Err(e) => {
                error!(error = %e, "Validation failed");
                return self.handle_validation_error(channels).await;
            }
        };

        // Process response
        if response.is_grant() {
            info!("Access granted");
            self.handle_access_granted(response, channels).await
        } else {
            info!("Access denied");
            self.handle_access_denied(response, channels).await
        }
    }

    /// Handle validation error.
    async fn handle_validation_error(&mut self, channels: &BroadcastChannels) -> Result<()> {
        // Create a deny response for error case
        let error_response = AccessResponse::deny("ERRO DE VALIDACAO".to_string());

        // Broadcast validation error event
        channels.broadcast_event(EmulatorEvent::ValidationResult(error_response));

        self.show_error_and_return_to_idle("ERRO DE VALIDACAO", "Tente novamente")
            .await
    }

    /// Handle access granted.
    async fn handle_access_granted(
        &mut self,
        response: AccessResponse,
        channels: &BroadcastChannels,
    ) -> Result<()> {
        // Transition to Granted state
        let old_state = *self.state_machine.current_state();
        self.state_machine.transition_to(TurnstileState::Granted)?;

        // Broadcast state transition
        channels.broadcast_state_transition(old_state, TurnstileState::Granted);

        // Show message on display
        let message = response.display_message();
        let timeout_secs = response.timeout_seconds();

        self.display
            .show_temporary(message, Duration::from_secs(timeout_secs as u64))?;

        info!(message = %message, timeout_secs, "Access granted");

        // Broadcast validation result
        channels.broadcast_event(EmulatorEvent::ValidationResult(response));

        // Wait for message display timeout
        sleep(Duration::from_secs(timeout_secs as u64)).await;

        // Simulate rotation
        self.simulate_rotation(channels).await
    }

    /// Handle access denied.
    async fn handle_access_denied(
        &mut self,
        response: AccessResponse,
        channels: &BroadcastChannels,
    ) -> Result<()> {
        // Transition to Denied state
        let old_state = *self.state_machine.current_state();
        self.state_machine.transition_to(TurnstileState::Denied)?;

        // Broadcast state transition
        channels.broadcast_state_transition(old_state, TurnstileState::Denied);

        // Show message on display
        let message = response.display_message();
        self.display.set_line(0, message)?;

        info!(message = %message, "Access denied");

        // Broadcast validation result
        channels.broadcast_event(EmulatorEvent::ValidationResult(response));

        // Wait before returning to idle
        sleep(Duration::from_secs(ACCESS_DENIED_DISPLAY_DURATION_SECS)).await;

        // Return to idle
        self.return_to_idle()
    }

    /// Simulate turnstile rotation.
    async fn simulate_rotation(&mut self, channels: &BroadcastChannels) -> Result<()> {
        // Transition to WaitingRotation state
        let old_state = *self.state_machine.current_state();
        self.state_machine
            .transition_to(TurnstileState::WaitingRotation)?;
        self.display.set_line(0, "AGUARDANDO GIRO...")?;

        // Broadcast state transition
        channels.broadcast_state_transition(old_state, TurnstileState::WaitingRotation);

        debug!("Waiting for rotation");

        // Send protocol status message to server (ONLINE mode only)
        if let Err(e) = self
            .send_status_message(TurnstileState::WaitingRotation, channels)
            .await
        {
            warn!(error = %e, "Failed to send WaitingRotation status message");
            // Non-fatal - continue with rotation simulation
        }

        // Simulate physical rotation delay
        sleep(Duration::from_secs(self.config.rotation_duration_secs)).await;

        // Transition to RotationCompleted state
        let old_state = *self.state_machine.current_state();
        self.state_machine
            .transition_to(TurnstileState::RotationCompleted)?;

        // Broadcast state transition
        channels.broadcast_state_transition(old_state, TurnstileState::RotationCompleted);

        info!("Rotation completed");

        // Send protocol status message to server (ONLINE mode only)
        if let Err(e) = self
            .send_status_message(TurnstileState::RotationCompleted, channels)
            .await
        {
            warn!(error = %e, "Failed to send RotationCompleted status message");
            // Non-fatal - continue with state transition
        }

        // Brief pause before returning to idle
        sleep(Duration::from_millis(POST_ROTATION_DELAY_MS)).await;

        // Return to idle
        self.return_to_idle()
    }

    /// Handle state timeout.
    async fn handle_state_timeout(&mut self, channels: &BroadcastChannels) -> Result<()> {
        let current_state = self.state_machine.current_state();
        warn!(?current_state, "State timeout occurred");

        match current_state {
            TurnstileState::WaitingRotation => {
                // User did not pass through in time
                let old_state = *self.state_machine.current_state();
                self.state_machine
                    .transition_to(TurnstileState::RotationTimeout)?;
                self.display.set_line(0, "TEMPO ESGOTADO")?;

                // Broadcast state transition
                channels.broadcast_state_transition(old_state, TurnstileState::RotationTimeout);

                // Broadcast display update
                let display_state = DisplayState::new(
                    self.display.get_line(0)?.to_string(),
                    self.display.get_line(1)?.to_string(),
                    self.config.mode,
                    self.device_id.as_u8(),
                    None,
                );
                channels.broadcast_display(display_state);

                info!("Rotation timeout");

                // Send protocol status message to server (ONLINE mode only)
                if let Err(e) = self
                    .send_status_message(TurnstileState::RotationTimeout, channels)
                    .await
                {
                    warn!(error = %e, "Failed to send RotationTimeout status message");
                    // Non-fatal - continue with recovery
                }

                sleep(Duration::from_secs(TIMEOUT_RECOVERY_DELAY_SECS)).await;
                self.return_to_idle_with_channels(channels)
            }
            _ => {
                // Unexpected timeout in other states - return to idle
                self.return_to_idle_with_channels(channels)
            }
        }
    }

    /// Return to idle state.
    fn return_to_idle(&mut self) -> Result<()> {
        debug!("Returning to idle state");

        self.state_machine.transition_to(TurnstileState::Idle)?;
        self.display.reset_to_default();
        self.keypad_buffer.clear();

        Ok(())
    }

    /// Return to idle state with channel broadcasting.
    ///
    /// This is the channel-aware version that broadcasts state and display
    /// updates to observers. Used internally by `run_with_channels`.
    fn return_to_idle_with_channels(&mut self, channels: &BroadcastChannels) -> Result<()> {
        debug!("Returning to idle state");

        // Transition to idle state
        let old_state = *self.state_machine.current_state();
        self.state_machine.transition_to(TurnstileState::Idle)?;
        self.display.reset_to_default();
        self.keypad_buffer.clear();

        // Broadcast state transition
        channels.broadcast_state_transition(old_state, TurnstileState::Idle);

        // Broadcast display update
        let display_state = DisplayState::new(
            self.display.get_line(0)?.to_string(),
            self.display.get_line(1)?.to_string(),
            self.config.mode,
            self.device_id.as_u8(),
            None,
        );
        channels.broadcast_display(display_state);

        // Broadcast buffer cleared event
        channels.broadcast_event(EmulatorEvent::KeypadBufferChanged(String::new()));

        Ok(())
    }

    /// Check if state timeout should be checked.
    fn should_check_timeout(&self) -> bool {
        matches!(
            self.state_machine.current_state(),
            TurnstileState::WaitingRotation
        )
    }

    /// Get timeout duration for current state.
    fn get_state_timeout_duration(&self) -> Duration {
        match self.state_machine.current_state() {
            TurnstileState::WaitingRotation => {
                Duration::from_secs(self.config.rotation_timeout_secs)
            }
            _ => Duration::from_secs(u64::MAX), // No timeout
        }
    }

    /// Get idle timeout duration.
    fn get_idle_timeout_duration(&self) -> Duration {
        let elapsed = self.last_activity.elapsed();
        let timeout = Duration::from_secs(self.config.idle_timeout_secs);

        if elapsed >= timeout {
            Duration::from_secs(0)
        } else {
            timeout - elapsed
        }
    }

    /// Handle idle timeout.
    fn handle_idle_timeout(&mut self, channels: &BroadcastChannels) -> Result<()> {
        // Only apply idle timeout when in Idle state with partial keypad input
        if matches!(self.state_machine.current_state(), TurnstileState::Idle)
            && !self.keypad_buffer.is_empty()
        {
            debug!(
                buffer_length = self.keypad_buffer.len(),
                "Idle timeout - clearing partial input"
            );
            self.keypad_buffer.clear();
            self.display.reset_to_default();

            // Broadcast buffer cleared event
            channels.broadcast_event(EmulatorEvent::KeypadBufferChanged(String::new()));

            // Broadcast display update
            let display_state = DisplayState::new(
                self.display.get_line(0)?.to_string(),
                self.display.get_line(1)?.to_string(),
                self.config.mode,
                self.device_id.as_u8(),
                None,
            );
            channels.broadcast_display(display_state);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    // Note: Full integration tests require setting up peripheral manager,
    // validator, and database. These will be in integration tests.

    #[test]
    fn test_append_digit() {
        // This test would require creating a full emulator instance
        // Moving to integration tests
    }

    #[test]
    fn test_clear_keypad_buffer() {
        // This test would require creating a full emulator instance
        // Moving to integration tests
    }
}
