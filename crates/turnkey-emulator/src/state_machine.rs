//! Turnstile state machine implementation.
//!
//! This module provides a complete state machine for managing turnstile
//! access control flow, from initial idle state through credential validation,
//! access grant/deny, and physical rotation completion.
//!
//! # States
//!
//! The state machine manages the following states:
//! - `Idle`: Waiting for user input
//! - `Reading`: Reading credential from peripheral device
//! - `Validating`: Validating credential with server or local database
//! - `Granted`: Access granted, displaying confirmation message
//! - `Denied`: Access denied, displaying rejection message
//! - `WaitingRotation`: Waiting for user to pass through turnstile
//! - `RotationInProgress`: Physical rotation happening
//! - `RotationCompleted`: User passed through successfully
//! - `RotationTimeout`: User did not pass within timeout period
//!
//! # Valid Transitions
//!
//! - Idle → Reading → Validating → Granted/Denied
//! - Granted → WaitingRotation → RotationInProgress → RotationCompleted → Idle
//! - WaitingRotation → RotationTimeout → Idle
//! - Denied → Idle
//!
//! # Protocol Mapping
//!
//! The state machine maps to Henry protocol command codes:
//! - `WaitingRotation` → `000+80` (waiting for rotation)
//! - `RotationCompleted` → `000+81` (rotation completed)
//! - `RotationTimeout` → `000+82` (rotation abandoned/timeout)
//!
//! # Examples
//!
//! ```
//! use turnkey_emulator::{StateMachine, TurnstileState};
//!
//! let mut machine = StateMachine::new();
//! assert_eq!(machine.current_state(), &TurnstileState::Idle);
//!
//! // Valid transition
//! machine.transition_to(TurnstileState::Reading).unwrap();
//! assert_eq!(machine.current_state(), &TurnstileState::Reading);
//! ```
//!
//! # Builder Pattern
//!
//! For advanced use cases like crash recovery:
//!
//! ```
//! use turnkey_emulator::{StateMachine, TurnstileState};
//! use std::time::Duration;
//!
//! let machine = StateMachine::builder()
//!     .with_initial_state(TurnstileState::WaitingRotation)
//!     .with_timeout(Duration::from_secs(10))
//!     .build();
//!
//! assert_eq!(machine.current_state(), &TurnstileState::WaitingRotation);
//! ```

use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use turnkey_core::{Error, Result};

/// Maximum number of state transitions to keep in history.
///
/// This value is chosen to balance memory usage with debugging capability:
/// - Each transition is approximately 32 bytes (2 enums + Instant + padding)
/// - 100 transitions equals approximately 3.2KB memory per state machine
/// - Typical access flow has 7 states, so 100 transitions equals approximately 14 complete flows
/// - Sufficient for debugging access patterns without excessive memory use
///
/// For high-throughput scenarios (greater than 1000 accesses per hour), consider adjusting
/// this value based on operational requirements and available memory.
const MAX_HISTORY_SIZE: usize = 100;

/// Represents all possible states in the turnstile access control flow.
///
/// Each state corresponds to a specific phase in the access control process,
/// from initial credential presentation through physical turnstile rotation.
///
/// # Protocol Mapping
///
/// Some states map directly to Henry protocol command codes:
/// - `WaitingRotation` emits `000+80` when entered
/// - `RotationCompleted` emits `000+81` when entered
/// - `RotationTimeout` emits `000+82` when entered
///
/// Use [`protocol_command_code`](TurnstileState::protocol_command_code) to get
/// the protocol code for states that emit messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnstileState {
    /// Waiting for user input or credential presentation.
    Idle,

    /// Reading credential from peripheral device (RFID, keypad, biometric).
    Reading,

    /// Validating credential with server or local database.
    Validating,

    /// Access granted, displaying confirmation message to user.
    Granted,

    /// Access denied, displaying rejection message to user.
    Denied,

    /// Waiting for user to physically pass through the turnstile.
    ///
    /// Protocol: Emits `000+80` (waiting for rotation) when entering this state.
    WaitingRotation,

    /// Physical rotation of turnstile in progress.
    RotationInProgress,

    /// User successfully passed through, rotation completed.
    ///
    /// Protocol: Emits `000+81` (rotation completed) when entering this state.
    RotationCompleted,

    /// User did not pass through within allowed timeout period.
    ///
    /// Protocol: Emits `000+82` (rotation abandoned/timeout) when entering this state.
    RotationTimeout,
}

impl fmt::Display for TurnstileState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_str = match self {
            TurnstileState::Idle => "Idle",
            TurnstileState::Reading => "Reading",
            TurnstileState::Validating => "Validating",
            TurnstileState::Granted => "Granted",
            TurnstileState::Denied => "Denied",
            TurnstileState::WaitingRotation => "WaitingRotation",
            TurnstileState::RotationInProgress => "RotationInProgress",
            TurnstileState::RotationCompleted => "RotationCompleted",
            TurnstileState::RotationTimeout => "RotationTimeout",
        };
        write!(f, "{}", state_str)
    }
}

impl TurnstileState {
    /// Check if transition to target state is valid from this state.
    ///
    /// This method implements the state machine transition rules, ensuring
    /// only valid state flows are allowed. Invalid transitions will cause
    /// errors when attempted via [`StateMachine::transition_to`].
    ///
    /// # Arguments
    ///
    /// * `target` - The target state to transition to
    ///
    /// # Returns
    ///
    /// Returns `true` if the transition is valid, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::TurnstileState;
    ///
    /// assert!(TurnstileState::Idle.can_transition_to(&TurnstileState::Reading));
    /// assert!(!TurnstileState::Idle.can_transition_to(&TurnstileState::Granted));
    /// ```
    pub fn can_transition_to(&self, target: &TurnstileState) -> bool {
        matches!(
            (self, target),
            // From Idle
            (TurnstileState::Idle, TurnstileState::Reading)
            // From Reading
            | (TurnstileState::Reading, TurnstileState::Validating)
            // From Validating
            | (TurnstileState::Validating, TurnstileState::Granted | TurnstileState::Denied)
            // From Granted
            | (TurnstileState::Granted, TurnstileState::WaitingRotation)
            // From Denied
            | (TurnstileState::Denied, TurnstileState::Idle)
            // From WaitingRotation
            | (TurnstileState::WaitingRotation, TurnstileState::RotationInProgress | TurnstileState::RotationTimeout)
            // From RotationInProgress
            | (TurnstileState::RotationInProgress, TurnstileState::RotationCompleted)
            // From RotationCompleted
            | (TurnstileState::RotationCompleted, TurnstileState::Idle)
            // From RotationTimeout
            | (TurnstileState::RotationTimeout, TurnstileState::Idle)
        )
    }

    /// Get the Henry protocol command code for this state, if applicable.
    ///
    /// Returns the protocol command code that should be emitted when
    /// entering this state, according to Henry protocol section 2.1-2.2.
    ///
    /// # Protocol Mapping
    ///
    /// - `WaitingRotation` → `000+80` (waiting for rotation)
    /// - `RotationCompleted` → `000+81` (rotation completed)
    /// - `RotationTimeout` → `000+82` (rotation abandoned/timeout)
    ///
    /// # Returns
    ///
    /// Returns `Some(command_code)` for states that emit protocol messages,
    /// `None` for internal states that do not emit messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::TurnstileState;
    ///
    /// assert_eq!(
    ///     TurnstileState::WaitingRotation.protocol_command_code(),
    ///     Some("000+80")
    /// );
    /// assert_eq!(
    ///     TurnstileState::RotationCompleted.protocol_command_code(),
    ///     Some("000+81")
    /// );
    /// assert_eq!(TurnstileState::Idle.protocol_command_code(), None);
    /// ```
    pub fn protocol_command_code(&self) -> Option<&'static str> {
        match self {
            TurnstileState::WaitingRotation => Some("000+80"),
            TurnstileState::RotationCompleted => Some("000+81"),
            TurnstileState::RotationTimeout => Some("000+82"),
            _ => None,
        }
    }

    /// Check if this state requires protocol message emission.
    ///
    /// # Returns
    ///
    /// Returns `true` if entering this state requires sending a protocol message.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::TurnstileState;
    ///
    /// assert!(TurnstileState::WaitingRotation.emits_protocol_message());
    /// assert!(!TurnstileState::Idle.emits_protocol_message());
    /// ```
    pub fn emits_protocol_message(&self) -> bool {
        self.protocol_command_code().is_some()
    }
}

/// Represents a single state transition with timestamp.
///
/// This struct records when a state transition occurred, useful for
/// audit trails, debugging, and performance analysis.
///
/// # Serialization Note
///
/// The `timestamp` field is not serialized as `Instant` is process-specific.
/// When deserializing, the timestamp will be set to the current time.
/// For persistent storage, use wall-clock time (SystemTime) in your application layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// The state transitioned from.
    pub from: TurnstileState,

    /// The state transitioned to.
    pub to: TurnstileState,

    /// When the transition occurred.
    ///
    /// Note: This field is not serialized. Upon deserialization, it will be set to
    /// the time of deserialization, not the original transition time.
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
}

impl StateTransition {
    /// Create a new state transition record.
    ///
    /// # Arguments
    ///
    /// * `from` - The state being transitioned from
    /// * `to` - The state being transitioned to
    ///
    /// # Returns
    ///
    /// Returns a new `StateTransition` with current timestamp.
    pub fn new(from: TurnstileState, to: TurnstileState) -> Self {
        Self {
            from,
            to,
            timestamp: Instant::now(),
        }
    }

    /// Get the duration since this transition occurred.
    ///
    /// # Returns
    ///
    /// Returns the elapsed time since the transition.
    pub fn elapsed(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// State machine for managing turnstile access control flow.
///
/// The state machine enforces valid state transitions, tracks state history,
/// and manages timeouts for time-sensitive states.
///
/// # Thread Safety
///
/// This struct is not thread-safe by design. In async contexts, protect
/// access using tokio::sync::Mutex or similar synchronization primitive.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::{StateMachine, TurnstileState};
///
/// let mut machine = StateMachine::new();
///
/// // Perform valid transitions
/// machine.transition_to(TurnstileState::Reading).unwrap();
/// machine.transition_to(TurnstileState::Validating).unwrap();
/// machine.transition_to(TurnstileState::Granted).unwrap();
///
/// // Check history
/// assert_eq!(machine.history().len(), 3);
/// ```
pub struct StateMachine {
    /// Current state of the turnstile.
    current_state: TurnstileState,

    /// When the current state was entered.
    state_entered_at: Instant,

    /// History of state transitions (limited to MAX_HISTORY_SIZE).
    history: VecDeque<StateTransition>,

    /// Optional timeout duration for the current state.
    current_timeout: Option<Duration>,
}

impl StateMachine {
    /// Create a new state machine in the Idle state.
    ///
    /// # Returns
    ///
    /// Returns a new `StateMachine` initialized to `Idle` state.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::{StateMachine, TurnstileState};
    ///
    /// let machine = StateMachine::new();
    /// assert_eq!(machine.current_state(), &TurnstileState::Idle);
    /// ```
    pub fn new() -> Self {
        Self {
            current_state: TurnstileState::Idle,
            state_entered_at: Instant::now(),
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            current_timeout: None,
        }
    }

    /// Create a builder for constructing a state machine with custom configuration.
    ///
    /// This is useful for advanced scenarios like crash recovery where you need
    /// to restore a machine to a specific state with pre-populated history.
    ///
    /// # Returns
    ///
    /// Returns a new `StateMachineBuilder` for fluent configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::{StateMachine, TurnstileState};
    /// use std::time::Duration;
    ///
    /// let machine = StateMachine::builder()
    ///     .with_initial_state(TurnstileState::WaitingRotation)
    ///     .with_timeout(Duration::from_secs(10))
    ///     .build();
    ///
    /// assert_eq!(machine.current_state(), &TurnstileState::WaitingRotation);
    /// ```
    pub fn builder() -> StateMachineBuilder {
        StateMachineBuilder::default()
    }

    /// Get the current state of the machine.
    ///
    /// # Returns
    ///
    /// Returns a reference to the current `TurnstileState`.
    pub fn current_state(&self) -> &TurnstileState {
        &self.current_state
    }

    /// Get the time elapsed in the current state.
    ///
    /// # Returns
    ///
    /// Returns the duration since entering the current state.
    pub fn time_in_current_state(&self) -> Duration {
        self.state_entered_at.elapsed()
    }

    /// Check if the current state has timed out.
    ///
    /// # Returns
    ///
    /// Returns `true` if a timeout is set and has been exceeded, `false` otherwise.
    pub fn has_timed_out(&self) -> bool {
        self.current_timeout
            .is_some_and(|timeout| self.time_in_current_state() >= timeout)
    }

    /// Get the remaining time before timeout, if any.
    ///
    /// # Returns
    ///
    /// Returns `Some(Duration)` with remaining time if timeout is set,
    /// `None` if no timeout or already timed out.
    pub fn time_remaining(&self) -> Option<Duration> {
        self.current_timeout.and_then(|timeout| {
            let elapsed = self.time_in_current_state();
            timeout.checked_sub(elapsed)
        })
    }

    /// Set a timeout for the current state.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The timeout duration
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use turnkey_emulator::StateMachine;
    ///
    /// let mut machine = StateMachine::new();
    /// machine.set_timeout(Duration::from_secs(5));
    /// assert!(!machine.has_timed_out());
    /// ```
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.current_timeout = Some(timeout);
    }

    /// Clear the timeout for the current state.
    pub fn clear_timeout(&mut self) {
        self.current_timeout = None;
    }

    /// Get a reference to the state transition history.
    ///
    /// # Returns
    ///
    /// Returns a reference to the deque of recent state transitions,
    /// ordered from oldest to newest.
    pub fn history(&self) -> &VecDeque<StateTransition> {
        &self.history
    }

    /// Get the last N state transitions.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of transitions to return
    ///
    /// # Returns
    ///
    /// Returns a vector of the most recent transitions, up to `count`.
    pub fn last_transitions(&self, count: usize) -> Vec<StateTransition> {
        self.history
            .iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }

    /// Transition to a new state, validating the transition.
    ///
    /// This method validates that the requested transition is legal according
    /// to the state machine rules. If valid, it updates the current state,
    /// records the transition in history, and returns the transition record.
    ///
    /// # Arguments
    ///
    /// * `new_state` - The target state to transition to
    ///
    /// # Returns
    ///
    /// Returns `Ok(StateTransition)` if the transition is valid,
    /// or `Err(Error::InvalidStateTransition)` if invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The requested transition is not valid for the current state
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::{StateMachine, TurnstileState};
    ///
    /// let mut machine = StateMachine::new();
    ///
    /// // Valid transition
    /// let transition = machine.transition_to(TurnstileState::Reading).unwrap();
    /// assert_eq!(transition.from, TurnstileState::Idle);
    /// assert_eq!(transition.to, TurnstileState::Reading);
    ///
    /// // Invalid transition
    /// let result = machine.transition_to(TurnstileState::Granted);
    /// assert!(result.is_err());
    /// ```
    pub fn transition_to(&mut self, new_state: TurnstileState) -> Result<StateTransition> {
        // Validate transition before making any changes
        if !self.current_state.can_transition_to(&new_state) {
            return Err(Error::InvalidStateTransition {
                from: self.current_state.to_string(),
                to: new_state.to_string(),
            });
        }

        // Create transition record before state change
        let transition = StateTransition::new(self.current_state, new_state);

        // Update state atomically
        self.perform_state_change(new_state, transition.clone());

        Ok(transition)
    }

    /// Check for timeout and automatically transition to timeout state if needed.
    ///
    /// This is a convenience method that combines timeout checking with
    /// automatic state transition for the common pattern of handling timeouts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(transition))` if a timeout occurred and transition succeeded.
    /// Returns `Ok(None)` if no timeout has occurred or current state has no timeout state.
    /// Returns `Err` if timeout occurred but transition to timeout state is invalid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use turnkey_emulator::{StateMachine, TurnstileState};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut machine = StateMachine::new();
    /// // Navigate to WaitingRotation state
    /// machine.transition_to(TurnstileState::Reading)?;
    /// machine.transition_to(TurnstileState::Validating)?;
    /// machine.transition_to(TurnstileState::Granted)?;
    /// machine.transition_to(TurnstileState::WaitingRotation)?;
    /// machine.set_timeout(Duration::from_secs(5));
    ///
    /// // In your event loop:
    /// if let Some(transition) = machine.check_and_handle_timeout()? {
    ///     println!("Timeout occurred: {:?}", transition);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn check_and_handle_timeout(&mut self) -> Result<Option<StateTransition>> {
        if !self.has_timed_out() {
            return Ok(None);
        }

        // Determine appropriate timeout state based on current state
        let timeout_state = match self.current_state {
            TurnstileState::WaitingRotation => TurnstileState::RotationTimeout,
            // Future: Could add ValidationTimeout state for Validating state
            _ => return Ok(None),
        };

        let transition = self.transition_to(timeout_state)?;
        Ok(Some(transition))
    }

    /// Reset the state machine to Idle state.
    ///
    /// This forcefully resets the machine to Idle regardless of current state.
    /// This should be used for error recovery or system resets.
    ///
    /// # Returns
    ///
    /// Returns a transition record for the reset.
    pub fn reset(&mut self) -> StateTransition {
        let transition = StateTransition::new(self.current_state, TurnstileState::Idle);
        self.perform_state_change(TurnstileState::Idle, transition.clone());
        transition
    }

    /// Internal method to perform state change and update all related fields.
    ///
    /// This method encapsulates the state change logic to avoid duplication
    /// between transition_to and reset methods (DRY principle).
    fn perform_state_change(&mut self, new_state: TurnstileState, transition: StateTransition) {
        self.current_state = new_state;
        self.state_entered_at = Instant::now();
        self.current_timeout = None;

        // Add to history with size limit enforcement
        self.add_to_history(transition);
    }

    /// Add a transition to history, maintaining size limit.
    ///
    /// This method encapsulates history management logic (Single Responsibility).
    fn add_to_history(&mut self, transition: StateTransition) {
        self.history.push_back(transition);
        if self.history.len() > MAX_HISTORY_SIZE {
            self.history.pop_front();
        }
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing `StateMachine` instances with custom configuration.
///
/// This builder is useful for advanced scenarios like crash recovery where
/// you need to restore a machine to a specific state with pre-populated history.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::{StateMachine, TurnstileState};
/// use std::time::Duration;
///
/// let machine = StateMachine::builder()
///     .with_initial_state(TurnstileState::WaitingRotation)
///     .with_timeout(Duration::from_secs(10))
///     .build();
///
/// assert_eq!(machine.current_state(), &TurnstileState::WaitingRotation);
/// assert!(machine.time_remaining().is_some());
/// ```
#[derive(Debug)]
pub struct StateMachineBuilder {
    initial_state: TurnstileState,
    history: VecDeque<StateTransition>,
    timeout: Option<Duration>,
}

impl StateMachineBuilder {
    /// Set the initial state for the machine.
    ///
    /// # Arguments
    ///
    /// * `state` - The initial state to start in
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn with_initial_state(mut self, state: TurnstileState) -> Self {
        self.initial_state = state;
        self
    }

    /// Set pre-populated history for the machine.
    ///
    /// # Arguments
    ///
    /// * `history` - A deque of historical transitions
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn with_history(mut self, history: VecDeque<StateTransition>) -> Self {
        self.history = history;
        self
    }

    /// Set an initial timeout for the machine.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The timeout duration
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the state machine with configured parameters.
    ///
    /// # Returns
    ///
    /// Returns a new `StateMachine` with the specified configuration.
    pub fn build(self) -> StateMachine {
        StateMachine {
            current_state: self.initial_state,
            state_entered_at: Instant::now(),
            history: self.history,
            current_timeout: self.timeout,
        }
    }
}

impl Default for StateMachineBuilder {
    fn default() -> Self {
        Self {
            initial_state: TurnstileState::Idle,
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            timeout: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_machine_starts_idle() {
        let machine = StateMachine::new();
        assert_eq!(machine.current_state(), &TurnstileState::Idle);
        assert_eq!(machine.history().len(), 0);
    }

    #[test]
    fn test_valid_transition_idle_to_reading() {
        let mut machine = StateMachine::new();
        let result = machine.transition_to(TurnstileState::Reading);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::Reading);

        let transition = result.unwrap();
        assert_eq!(transition.from, TurnstileState::Idle);
        assert_eq!(transition.to, TurnstileState::Reading);
    }

    #[test]
    fn test_valid_transition_reading_to_validating() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        let result = machine.transition_to(TurnstileState::Validating);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::Validating);
    }

    #[test]
    fn test_valid_transition_validating_to_granted() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        let result = machine.transition_to(TurnstileState::Granted);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::Granted);
    }

    #[test]
    fn test_valid_transition_validating_to_denied() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        let result = machine.transition_to(TurnstileState::Denied);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::Denied);
    }

    #[test]
    fn test_valid_transition_granted_to_waiting_rotation() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        let result = machine.transition_to(TurnstileState::WaitingRotation);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::WaitingRotation);
    }

    #[test]
    fn test_valid_transition_waiting_to_rotation_in_progress() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        let result = machine.transition_to(TurnstileState::RotationInProgress);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::RotationInProgress);
    }

    #[test]
    fn test_valid_transition_rotation_in_progress_to_completed() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        machine
            .transition_to(TurnstileState::RotationInProgress)
            .unwrap();
        let result = machine.transition_to(TurnstileState::RotationCompleted);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::RotationCompleted);
    }

    #[test]
    fn test_valid_transition_rotation_completed_to_idle() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        machine
            .transition_to(TurnstileState::RotationInProgress)
            .unwrap();
        machine
            .transition_to(TurnstileState::RotationCompleted)
            .unwrap();
        let result = machine.transition_to(TurnstileState::Idle);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::Idle);
    }

    #[test]
    fn test_valid_transition_waiting_to_rotation_timeout() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        let result = machine.transition_to(TurnstileState::RotationTimeout);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::RotationTimeout);
    }

    #[test]
    fn test_valid_transition_rotation_timeout_to_idle() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        machine
            .transition_to(TurnstileState::RotationTimeout)
            .unwrap();
        let result = machine.transition_to(TurnstileState::Idle);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::Idle);
    }

    #[test]
    fn test_valid_transition_denied_to_idle() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Denied).unwrap();
        let result = machine.transition_to(TurnstileState::Idle);

        assert!(result.is_ok());
        assert_eq!(machine.current_state(), &TurnstileState::Idle);
    }

    #[test]
    fn test_invalid_transition_idle_to_validating() {
        let mut machine = StateMachine::new();
        let result = machine.transition_to(TurnstileState::Validating);

        assert!(result.is_err());
        assert_eq!(machine.current_state(), &TurnstileState::Idle);
    }

    #[test]
    fn test_invalid_transition_idle_to_granted() {
        let mut machine = StateMachine::new();
        let result = machine.transition_to(TurnstileState::Granted);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_transition_reading_to_granted() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        let result = machine.transition_to(TurnstileState::Granted);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_transition_granted_to_idle() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        let result = machine.transition_to(TurnstileState::Idle);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_transition_denied_to_granted() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Denied).unwrap();
        let result = machine.transition_to(TurnstileState::Granted);

        assert!(result.is_err());
    }

    #[test]
    fn test_transition_history_is_recorded() {
        let mut machine = StateMachine::new();

        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();

        assert_eq!(machine.history().len(), 3);

        let history: Vec<_> = machine.history().iter().collect();
        assert_eq!(history[0].from, TurnstileState::Idle);
        assert_eq!(history[0].to, TurnstileState::Reading);
        assert_eq!(history[1].from, TurnstileState::Reading);
        assert_eq!(history[1].to, TurnstileState::Validating);
        assert_eq!(history[2].from, TurnstileState::Validating);
        assert_eq!(history[2].to, TurnstileState::Granted);
    }

    #[test]
    fn test_last_transitions_returns_most_recent() {
        let mut machine = StateMachine::new();

        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();

        let last_two = machine.last_transitions(2);
        assert_eq!(last_two.len(), 2);
        assert_eq!(last_two[0].from, TurnstileState::Reading);
        assert_eq!(last_two[1].from, TurnstileState::Validating);
    }

    #[test]
    fn test_timeout_tracking() {
        let mut machine = StateMachine::new();
        machine.set_timeout(Duration::from_millis(100));

        assert!(!machine.has_timed_out());
        assert!(machine.time_remaining().is_some());

        thread::sleep(Duration::from_millis(150));

        assert!(machine.has_timed_out());
        assert!(machine.time_remaining().is_none());
    }

    #[test]
    fn test_timeout_cleared_on_transition() {
        let mut machine = StateMachine::new();
        machine.set_timeout(Duration::from_secs(5));

        machine.transition_to(TurnstileState::Reading).unwrap();

        assert!(machine.time_remaining().is_none());
        assert!(!machine.has_timed_out());
    }

    #[test]
    fn test_time_in_current_state() {
        let machine = StateMachine::new();

        thread::sleep(Duration::from_millis(50));
        let elapsed = machine.time_in_current_state();

        assert!(elapsed >= Duration::from_millis(50));
        assert!(elapsed < Duration::from_millis(200));
    }

    #[test]
    fn test_reset_returns_to_idle() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();

        let transition = machine.reset();

        assert_eq!(machine.current_state(), &TurnstileState::Idle);
        assert_eq!(transition.from, TurnstileState::Validating);
        assert_eq!(transition.to, TurnstileState::Idle);
    }

    #[test]
    fn test_state_display_formatting() {
        assert_eq!(TurnstileState::Idle.to_string(), "Idle");
        assert_eq!(TurnstileState::Reading.to_string(), "Reading");
        assert_eq!(TurnstileState::Validating.to_string(), "Validating");
        assert_eq!(TurnstileState::Granted.to_string(), "Granted");
        assert_eq!(TurnstileState::Denied.to_string(), "Denied");
        assert_eq!(
            TurnstileState::WaitingRotation.to_string(),
            "WaitingRotation"
        );
        assert_eq!(
            TurnstileState::RotationInProgress.to_string(),
            "RotationInProgress"
        );
        assert_eq!(
            TurnstileState::RotationCompleted.to_string(),
            "RotationCompleted"
        );
        assert_eq!(
            TurnstileState::RotationTimeout.to_string(),
            "RotationTimeout"
        );
    }

    #[test]
    fn test_complete_access_granted_flow() {
        let mut machine = StateMachine::new();

        // Complete successful access flow
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        machine
            .transition_to(TurnstileState::RotationInProgress)
            .unwrap();
        machine
            .transition_to(TurnstileState::RotationCompleted)
            .unwrap();
        machine.transition_to(TurnstileState::Idle).unwrap();

        assert_eq!(machine.current_state(), &TurnstileState::Idle);
        assert_eq!(machine.history().len(), 7);
    }

    #[test]
    fn test_complete_access_denied_flow() {
        let mut machine = StateMachine::new();

        // Complete denied access flow
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Denied).unwrap();
        machine.transition_to(TurnstileState::Idle).unwrap();

        assert_eq!(machine.current_state(), &TurnstileState::Idle);
        assert_eq!(machine.history().len(), 4);
    }

    #[test]
    fn test_complete_rotation_timeout_flow() {
        let mut machine = StateMachine::new();

        // Flow with rotation timeout
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        machine
            .transition_to(TurnstileState::RotationTimeout)
            .unwrap();
        machine.transition_to(TurnstileState::Idle).unwrap();

        assert_eq!(machine.current_state(), &TurnstileState::Idle);
        assert_eq!(machine.history().len(), 6);
    }

    #[test]
    fn test_transition_elapsed_time() {
        let transition = StateTransition::new(TurnstileState::Idle, TurnstileState::Reading);

        thread::sleep(Duration::from_millis(50));

        let elapsed = transition.elapsed();
        assert!(elapsed >= Duration::from_millis(50));
        assert!(elapsed < Duration::from_millis(200));
    }

    #[test]
    fn test_history_size_limit() {
        let mut machine = StateMachine::new();

        // Perform more transitions than MAX_HISTORY_SIZE
        for _ in 0..150 {
            machine.transition_to(TurnstileState::Reading).unwrap();
            machine.transition_to(TurnstileState::Validating).unwrap();
            machine.transition_to(TurnstileState::Denied).unwrap();
            machine.transition_to(TurnstileState::Idle).unwrap();
        }

        // History should be capped at MAX_HISTORY_SIZE
        assert!(machine.history().len() <= MAX_HISTORY_SIZE);
        assert_eq!(machine.history().len(), MAX_HISTORY_SIZE);
    }

    #[test]
    fn test_can_transition_to_method() {
        // Test a few key transitions
        assert!(TurnstileState::Idle.can_transition_to(&TurnstileState::Reading));
        assert!(!TurnstileState::Idle.can_transition_to(&TurnstileState::Granted));

        assert!(TurnstileState::Reading.can_transition_to(&TurnstileState::Validating));
        assert!(!TurnstileState::Reading.can_transition_to(&TurnstileState::Idle));

        assert!(TurnstileState::Validating.can_transition_to(&TurnstileState::Granted));
        assert!(TurnstileState::Validating.can_transition_to(&TurnstileState::Denied));
        assert!(!TurnstileState::Validating.can_transition_to(&TurnstileState::Idle));
    }

    #[test]
    fn test_protocol_command_code_waiting_rotation() {
        assert_eq!(
            TurnstileState::WaitingRotation.protocol_command_code(),
            Some("000+80")
        );
    }

    #[test]
    fn test_protocol_command_code_rotation_completed() {
        assert_eq!(
            TurnstileState::RotationCompleted.protocol_command_code(),
            Some("000+81")
        );
    }

    #[test]
    fn test_protocol_command_code_rotation_timeout() {
        assert_eq!(
            TurnstileState::RotationTimeout.protocol_command_code(),
            Some("000+82")
        );
    }

    #[test]
    fn test_protocol_command_code_none_for_internal_states() {
        assert_eq!(TurnstileState::Idle.protocol_command_code(), None);
        assert_eq!(TurnstileState::Reading.protocol_command_code(), None);
        assert_eq!(TurnstileState::Validating.protocol_command_code(), None);
        assert_eq!(TurnstileState::Granted.protocol_command_code(), None);
        assert_eq!(TurnstileState::Denied.protocol_command_code(), None);
        assert_eq!(
            TurnstileState::RotationInProgress.protocol_command_code(),
            None
        );
    }

    #[test]
    fn test_emits_protocol_message() {
        assert!(TurnstileState::WaitingRotation.emits_protocol_message());
        assert!(TurnstileState::RotationCompleted.emits_protocol_message());
        assert!(TurnstileState::RotationTimeout.emits_protocol_message());

        assert!(!TurnstileState::Idle.emits_protocol_message());
        assert!(!TurnstileState::Reading.emits_protocol_message());
        assert!(!TurnstileState::Validating.emits_protocol_message());
    }

    #[test]
    fn test_builder_default() {
        let machine = StateMachine::builder().build();

        assert_eq!(machine.current_state(), &TurnstileState::Idle);
        assert_eq!(machine.history().len(), 0);
        assert!(machine.time_remaining().is_none());
    }

    #[test]
    fn test_builder_with_initial_state() {
        let machine = StateMachine::builder()
            .with_initial_state(TurnstileState::WaitingRotation)
            .build();

        assert_eq!(machine.current_state(), &TurnstileState::WaitingRotation);
    }

    #[test]
    fn test_builder_with_timeout() {
        let machine = StateMachine::builder()
            .with_timeout(Duration::from_secs(10))
            .build();

        assert!(machine.time_remaining().is_some());
        assert!(!machine.has_timed_out());
    }

    #[test]
    fn test_builder_with_history() {
        let mut history = VecDeque::new();
        history.push_back(StateTransition::new(
            TurnstileState::Idle,
            TurnstileState::Reading,
        ));

        let machine = StateMachine::builder().with_history(history).build();

        assert_eq!(machine.history().len(), 1);
    }

    #[test]
    fn test_builder_fluent_api() {
        let machine = StateMachine::builder()
            .with_initial_state(TurnstileState::Granted)
            .with_timeout(Duration::from_secs(5))
            .build();

        assert_eq!(machine.current_state(), &TurnstileState::Granted);
        assert!(machine.time_remaining().is_some());
    }

    #[test]
    fn test_check_and_handle_timeout_no_timeout() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();

        let result = machine.check_and_handle_timeout().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_check_and_handle_timeout_not_expired() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        machine.set_timeout(Duration::from_secs(10));

        let result = machine.check_and_handle_timeout().unwrap();
        assert!(result.is_none());
        assert_eq!(machine.current_state(), &TurnstileState::WaitingRotation);
    }

    #[test]
    fn test_check_and_handle_timeout_expired() {
        let mut machine = StateMachine::new();
        machine.transition_to(TurnstileState::Reading).unwrap();
        machine.transition_to(TurnstileState::Validating).unwrap();
        machine.transition_to(TurnstileState::Granted).unwrap();
        machine
            .transition_to(TurnstileState::WaitingRotation)
            .unwrap();
        machine.set_timeout(Duration::from_millis(50));

        thread::sleep(Duration::from_millis(100));

        let result = machine.check_and_handle_timeout().unwrap();
        assert!(result.is_some());

        let transition = result.unwrap();
        assert_eq!(transition.from, TurnstileState::WaitingRotation);
        assert_eq!(transition.to, TurnstileState::RotationTimeout);
        assert_eq!(machine.current_state(), &TurnstileState::RotationTimeout);
    }

    #[test]
    fn test_check_and_handle_timeout_unsupported_state() {
        let mut machine = StateMachine::new();
        machine.set_timeout(Duration::from_millis(50));

        thread::sleep(Duration::from_millis(100));

        let result = machine.check_and_handle_timeout().unwrap();
        assert!(result.is_none());
        assert_eq!(machine.current_state(), &TurnstileState::Idle);
    }

    #[test]
    fn test_state_serialization() {
        let state = TurnstileState::WaitingRotation;
        let serialized = serde_json::to_string(&state).unwrap();
        assert_eq!(serialized, "\"waiting_rotation\"");

        let deserialized: TurnstileState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, state);
    }

    #[test]
    fn test_transition_serialization() {
        let transition = StateTransition::new(TurnstileState::Idle, TurnstileState::Reading);
        let serialized = serde_json::to_string(&transition).unwrap();

        // Should serialize from and to states
        assert!(serialized.contains("\"from\""));
        assert!(serialized.contains("\"to\""));
        assert!(serialized.contains("\"idle\""));
        assert!(serialized.contains("\"reading\""));

        let deserialized: StateTransition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.from, TurnstileState::Idle);
        assert_eq!(deserialized.to, TurnstileState::Reading);
    }
}
