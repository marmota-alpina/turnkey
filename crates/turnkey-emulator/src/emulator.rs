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

use tokio::time::sleep;
use tracing::{debug, error, info, trace, warn};

use turnkey_core::{DeviceId, HenryTimestamp, ReaderType, Result};
use turnkey_hardware::manager::{PeripheralEvent, PeripheralHandle};
use turnkey_hardware::{BiometricData, KeypadInput};
use turnkey_protocol::commands::access::{AccessRequest, AccessResponse};
use turnkey_storage::Validator;

use crate::TurnstileState;
use crate::config::EmulatorConfig;
use crate::display::VirtualDisplay;
use crate::state_machine::StateMachine;

/// Minimum credential length per Henry protocol specification.
const MIN_CREDENTIAL_LENGTH: usize = 3;

/// Maximum credential length per Henry protocol specification.
const MAX_CREDENTIAL_LENGTH: usize = 20;

/// Maximum keypad buffer length (same as max credential length).
const MAX_BUFFER_LENGTH: usize = MAX_CREDENTIAL_LENGTH;

/// Error display duration in seconds.
const ERROR_DISPLAY_DURATION_SECS: u64 = 2;

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

    /// Run the emulator event loop.
    ///
    /// This method runs indefinitely, processing events from peripherals
    /// and handling state transitions. It only returns on error or shutdown.
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
    pub async fn run(mut self) -> Result<()> {
        info!(
            device_id = %self.device_id,
            mode = ?self.config.mode,
            "Starting turnstile emulator"
        );

        loop {
            // Get timeout duration before select! to avoid borrow issues
            let timeout_duration = self.get_state_timeout_duration();
            let should_timeout = self.should_check_timeout();
            let idle_timeout = self.get_idle_timeout_duration();
            let should_check_idle = self.config.idle_timeout_secs > 0;

            tokio::select! {
                // Peripheral events
                Some(event) = self.peripheral_handle.recv() => {
                    if let Err(e) = self.handle_peripheral_event(event).await {
                        error!(error = %e, "Error handling peripheral event");
                        // Continue running - peripheral errors are recoverable
                    }
                }

                // State timeouts
                _ = sleep(timeout_duration), if should_timeout => {
                    if let Err(e) = self.handle_state_timeout().await {
                        error!(error = %e, "Error handling state timeout");
                        // Try to recover to Idle state
                        let _ = self.return_to_idle();
                    }
                }

                // Idle timeout
                _ = sleep(idle_timeout), if should_check_idle => {
                    if let Err(e) = self.handle_idle_timeout() {
                        error!(error = %e, "Error handling idle timeout");
                        let _ = self.return_to_idle();
                    }
                }
            }
        }
    }

    /// Handle peripheral event.
    ///
    /// Dispatches the event to the appropriate handler based on event type.
    async fn handle_peripheral_event(&mut self, event: PeripheralEvent) -> Result<()> {
        trace!(?event, "Received peripheral event");
        self.last_activity = Instant::now();

        match event {
            PeripheralEvent::KeypadInput(input) => self.handle_keypad_input(input).await,
            PeripheralEvent::CardRead(card) => {
                let credential = card.uid_decimal();
                info!(uid = %credential, "Card read");

                if !Self::is_valid_credential_length(&credential) {
                    warn!(
                        credential_length = credential.len(),
                        "Invalid card credential length (must be {}-{} characters)",
                        MIN_CREDENTIAL_LENGTH,
                        MAX_CREDENTIAL_LENGTH
                    );
                    return self
                        .show_error_and_return_to_idle("CARTAO INVALIDO", "Contacte admin")
                        .await;
                }

                self.validate_credential(credential, ReaderType::Rfid).await
            }
            PeripheralEvent::FingerprintCaptured(bio) => {
                info!(quality = bio.quality, "Fingerprint captured");
                self.handle_biometric_capture(bio).await
            }
            PeripheralEvent::DeviceError { device_type, error } => {
                warn!(%device_type, %error, "Device error");
                Ok(())
            }
            _ => {
                warn!("Unknown peripheral event");
                Ok(())
            }
        }
    }

    /// Handle keypad input.
    async fn handle_keypad_input(&mut self, input: KeypadInput) -> Result<()> {
        match input {
            KeypadInput::Digit(digit) => {
                self.append_digit(digit);
                Ok(())
            }
            KeypadInput::Enter => self.process_keypad_entry().await,
            KeypadInput::Cancel => {
                self.clear_keypad_buffer();
                Ok(())
            }
            KeypadInput::Clear => {
                self.clear_last_digit();
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

    /// Show error message on display and return to idle after delay.
    async fn show_error_and_return_to_idle(&mut self, line1: &str, line2: &str) -> Result<()> {
        let _ = self.display.set_line(0, line1);
        let _ = self.display.set_line(1, line2);
        sleep(Duration::from_secs(ERROR_DISPLAY_DURATION_SECS)).await;
        self.return_to_idle()
    }

    /// Append digit to keypad buffer.
    fn append_digit(&mut self, digit: u8) {
        if self.keypad_buffer.len() < MAX_BUFFER_LENGTH {
            // Convert digit (0-9) to char
            let digit_char = (b'0' + digit) as char;
            self.keypad_buffer.push(digit_char);
            debug!(buffer = %self.keypad_buffer, "Digit appended");

            // Update display to show buffer
            let display_text = format!("Codigo: {}", self.keypad_buffer);
            let _ = self.display.set_line(1, &display_text);
        }
    }

    /// Process keypad entry (ENTER pressed).
    async fn process_keypad_entry(&mut self) -> Result<()> {
        if self.keypad_buffer.is_empty() {
            debug!("Empty keypad buffer, ignoring ENTER");
            return Ok(());
        }

        let credential = self.keypad_buffer.clone();
        self.keypad_buffer.clear();

        if !Self::is_valid_credential_length(&credential) {
            warn!(
                credential_length = credential.len(),
                "Invalid keypad credential length (must be {}-{} characters)",
                MIN_CREDENTIAL_LENGTH,
                MAX_CREDENTIAL_LENGTH
            );
            return self
                .show_error_and_return_to_idle(
                    "CODIGO INVALIDO",
                    &format!(
                        "{}-{} digitos",
                        MIN_CREDENTIAL_LENGTH, MAX_CREDENTIAL_LENGTH
                    ),
                )
                .await;
        }

        info!(credential = %credential, "Processing keypad entry");
        self.validate_credential(credential, ReaderType::Rfid).await
    }

    /// Clear keypad buffer.
    fn clear_keypad_buffer(&mut self) {
        if !self.keypad_buffer.is_empty() {
            debug!("Clearing keypad buffer");
            self.keypad_buffer.clear();
            let _ = self.display.set_line(1, "");
        }
    }

    /// Clear last digit from keypad buffer.
    fn clear_last_digit(&mut self) {
        if !self.keypad_buffer.is_empty() {
            self.keypad_buffer.pop();
            debug!(buffer = %self.keypad_buffer, "Last digit cleared");

            let display_text = if self.keypad_buffer.is_empty() {
                String::new()
            } else {
                format!("Codigo: {}", self.keypad_buffer)
            };
            let _ = self.display.set_line(1, &display_text);
        }
    }

    /// Handle biometric capture.
    async fn handle_biometric_capture(&mut self, _bio: BiometricData) -> Result<()> {
        // For now, biometric validation is not implemented
        // In future: extract template and validate
        warn!("Biometric validation not yet implemented");
        Ok(())
    }

    /// Validate credential (card number or keypad code).
    async fn validate_credential(
        &mut self,
        credential: String,
        reader_type: ReaderType,
    ) -> Result<()> {
        // Transition to Validating state
        self.state_machine
            .transition_to(TurnstileState::Validating)?;
        self.display.update_from_state(&TurnstileState::Validating);

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

        // Validate (synchronous async call - blocks until response)
        let response = match self.validator.validate(&request).await {
            Ok(resp) => resp,
            Err(e) => {
                error!(error = %e, "Validation failed");
                return self.handle_validation_error().await;
            }
        };

        // Process response
        if response.is_grant() {
            info!("Access granted");
            self.handle_access_granted(response).await
        } else {
            info!("Access denied");
            self.handle_access_denied(response).await
        }
    }

    /// Handle validation error.
    async fn handle_validation_error(&mut self) -> Result<()> {
        self.show_error_and_return_to_idle("ERRO DE VALIDACAO", "Tente novamente")
            .await
    }

    /// Handle access granted.
    async fn handle_access_granted(&mut self, response: AccessResponse) -> Result<()> {
        // Transition to Granted state
        self.state_machine.transition_to(TurnstileState::Granted)?;

        // Show message on display
        let message = response.display_message();
        let timeout_secs = response.timeout_seconds();

        self.display
            .show_temporary(message, Duration::from_secs(timeout_secs as u64))?;

        info!(message = %message, timeout_secs, "Access granted");

        // Wait for message display timeout
        sleep(Duration::from_secs(timeout_secs as u64)).await;

        // Simulate rotation
        self.simulate_rotation().await
    }

    /// Handle access denied.
    async fn handle_access_denied(&mut self, response: AccessResponse) -> Result<()> {
        // Transition to Denied state
        self.state_machine.transition_to(TurnstileState::Denied)?;

        // Show message on display
        let message = response.display_message();
        self.display.set_line(0, message)?;

        info!(message = %message, "Access denied");

        // Wait before returning to idle
        sleep(Duration::from_secs(3)).await;

        // Return to idle
        self.return_to_idle()
    }

    /// Simulate turnstile rotation.
    async fn simulate_rotation(&mut self) -> Result<()> {
        // Transition to WaitingRotation state
        self.state_machine
            .transition_to(TurnstileState::WaitingRotation)?;
        self.display.set_line(0, "AGUARDANDO GIRO...")?;

        debug!("Waiting for rotation");

        // TODO: Send 000+80 protocol message to server (ONLINE mode only)
        //
        // Protocol format: <ID>+REON+000+80]<CARD>]<TIMESTAMP>]<DIRECTION>]<READER>]
        // Example: 15+REON+000+80]1234567890]20/10/2025 14:30:00]1]1]
        //
        // To implement:
        // 1. Check if in Online mode (config.mode == OperationMode::Online)
        // 2. Create TurnstileStatus with WaitingRotation state
        // 3. Build Message using MessageBuilder with CommandCode::WaitingRotation
        // 4. Send via validator's TCP client (requires adding send_status() method to Validator trait)
        //
        // Note: This requires architectural change to expose TCP client or add send method to Validator

        // Simulate physical rotation delay
        sleep(Duration::from_secs(2)).await;

        // Transition to RotationCompleted state
        self.state_machine
            .transition_to(TurnstileState::RotationCompleted)?;

        info!("Rotation completed");

        // TODO: Send 000+81 protocol message to server (ONLINE mode only)
        //
        // Protocol format: <ID>+REON+000+81]<CARD>]<TIMESTAMP>]<DIRECTION>]<READER>]
        // Example: 15+REON+000+81]1234567890]20/10/2025 14:30:02]1]1]
        //
        // Same implementation pattern as 000+80 above, using CommandCode::RotationCompleted

        // Brief pause before returning to idle
        sleep(Duration::from_millis(500)).await;

        // Return to idle
        self.return_to_idle()
    }

    /// Handle state timeout.
    async fn handle_state_timeout(&mut self) -> Result<()> {
        let current_state = self.state_machine.current_state();
        warn!(?current_state, "State timeout occurred");

        match current_state {
            TurnstileState::WaitingRotation => {
                // User did not pass through in time
                self.state_machine
                    .transition_to(TurnstileState::RotationTimeout)?;
                self.display.set_line(0, "TEMPO ESGOTADO")?;

                info!("Rotation timeout");

                // TODO: Send 000+82 protocol message to server (ONLINE mode only)
                //
                // Protocol format: <ID>+REON+000+82]<CARD>]<TIMESTAMP>]<DIRECTION>]<READER>]
                // Example: 15+REON+000+82]1234567890]20/10/2025 14:30:12]1]1]
                //
                // Same implementation pattern as 000+80, using CommandCode::RotationTimeout

                sleep(Duration::from_secs(2)).await;
                self.return_to_idle()
            }
            _ => {
                // Unexpected timeout in other states - return to idle
                self.return_to_idle()
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
    fn handle_idle_timeout(&mut self) -> Result<()> {
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
