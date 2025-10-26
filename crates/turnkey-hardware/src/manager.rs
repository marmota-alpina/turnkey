//! Peripheral device manager.
//!
//! This module provides the `PeripheralManager`, which coordinates multiple
//! peripheral devices and aggregates their events into a unified stream for
//! consumption by the emulator core.
//!
//! # Architecture
//!
//! The manager uses enum dispatch pattern to maintain zero-cost abstraction
//! while providing concrete type dispatch for device management. Each device
//! runs in its own async task and sends events to a shared channel.
//!
//! ```text
//! ┌──────────┐       ┌─────────────────┐
//! │ Keypad   │──────►│                 │
//! │ Task     │       │  Event Channel  │
//! └──────────┘       │  (mpsc)         │
//!                    │                 │──────► Emulator Core
//! ┌──────────┐       │                 │
//! │ RFID     │──────►│                 │
//! │ Task     │       └─────────────────┘
//! └──────────┘
//!
//! ┌──────────┐
//! │ Biometric│──────►
//! │ Task     │
//! └──────────┘
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
//! use turnkey_hardware::devices::AnyKeypadDevice;
//! use turnkey_hardware::mock::MockKeypad;
//!
//! #[tokio::main]
//! async fn main() -> turnkey_hardware::Result<()> {
//!     let config = PeripheralConfig::default();
//!     let mut manager = PeripheralManager::new(config);
//!
//!     // Register devices
//!     let (keypad, _handle) = MockKeypad::new();
//!     manager.register_keypad(AnyKeypadDevice::Mock(keypad));
//!
//!     // Start manager and get handle for receiving events
//!     let mut handle = manager.start();
//!
//!     // Receive events (in real usage, this would be in main loop)
//!     while let Some(event) = handle.recv().await {
//!         println!("Event: {:?}", event);
//!     }
//!
//!     // Shutdown when done
//!     handle.shutdown().await?;
//!     Ok(())
//! }
//! ```

use crate::devices::{AnyBiometricDevice, AnyKeypadDevice, AnyRfidDevice};
use crate::traits::{BiometricDevice, KeypadDevice, RfidDevice};
use crate::{BiometricData, CardData, KeypadInput, Result};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

/// Unified event from any peripheral device.
///
/// All peripheral devices send their events through this enum, allowing
/// the emulator core to handle input from multiple sources through a
/// single event stream.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PeripheralEvent {
    /// Input received from keypad.
    KeypadInput(KeypadInput),

    /// Card read from RFID reader.
    CardRead(CardData),

    /// Fingerprint captured from biometric scanner.
    FingerprintCaptured(BiometricData),

    /// Device error occurred.
    ///
    /// This event is sent when a device encounters an error. The device
    /// task will terminate after sending this event.
    DeviceError {
        /// Type of device that encountered the error.
        device_type: DeviceType,

        /// Error message.
        error: String,
    },
}

/// Type of peripheral device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    /// Keypad device.
    Keypad,

    /// RFID reader device.
    Rfid,

    /// Biometric scanner device.
    Biometric,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keypad => write!(f, "Keypad"),
            Self::Rfid => write!(f, "RFID"),
            Self::Biometric => write!(f, "Biometric"),
        }
    }
}

/// Configuration for peripheral devices.
///
/// Controls which devices are enabled and will be started by the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeripheralConfig {
    /// Enable keypad device.
    pub keypad_enabled: bool,

    /// Enable RFID reader device.
    pub rfid_enabled: bool,

    /// Enable biometric scanner device.
    pub biometric_enabled: bool,
}

impl Default for PeripheralConfig {
    fn default() -> Self {
        Self {
            keypad_enabled: true,
            rfid_enabled: true,
            biometric_enabled: false,
        }
    }
}

/// Statistics about connected peripherals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeripheralStats {
    /// Keypad is connected.
    pub keypad_connected: bool,

    /// RFID reader is connected.
    pub rfid_connected: bool,

    /// Biometric scanner is connected.
    pub biometric_connected: bool,
}

/// Handle for receiving events from peripheral devices.
///
/// This handle provides access to the event stream from all registered
/// peripheral devices. It can be held independently of the manager and
/// used to receive events in the main application loop.
///
/// # Examples
///
/// ```no_run
/// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig, PeripheralEvent};
///
/// # async fn example() -> turnkey_hardware::Result<()> {
/// let config = PeripheralConfig::default();
/// let mut manager = PeripheralManager::new(config);
///
/// // Start manager and get event handle
/// let mut handle = manager.start();
///
/// // Receive events from any device
/// while let Some(event) = handle.recv().await {
///     match event {
///         PeripheralEvent::KeypadInput(input) => {
///             println!("Keypad input: {:?}", input);
///         }
///         PeripheralEvent::CardRead(card) => {
///             println!("Card read: {}", card.uid_decimal());
///         }
///         _ => {}
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub struct PeripheralHandle {
    /// Event receiver for consuming events from devices.
    event_rx: mpsc::Receiver<PeripheralEvent>,

    /// Running device tasks.
    tasks: JoinSet<Result<()>>,
}

impl PeripheralHandle {
    /// Receive the next event from any peripheral device.
    ///
    /// This method blocks asynchronously until an event is available.
    /// Returns `None` when all device tasks have terminated and the
    /// channel is closed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_hardware::manager::{PeripheralHandle, PeripheralEvent};
    /// # async fn example(mut handle: PeripheralHandle) {
    /// while let Some(event) = handle.recv().await {
    ///     match event {
    ///         PeripheralEvent::KeypadInput(input) => {
    ///             println!("Key pressed: {:?}", input);
    ///         }
    ///         PeripheralEvent::CardRead(card) => {
    ///             println!("Card: {}", card.uid_hex());
    ///         }
    ///         PeripheralEvent::FingerprintCaptured(bio) => {
    ///             println!("Fingerprint quality: {}", bio.quality);
    ///         }
    ///         PeripheralEvent::DeviceError { device_type, error } => {
    ///             eprintln!("{} error: {}", device_type, error);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # }
    /// ```
    pub async fn recv(&mut self) -> Option<PeripheralEvent> {
        self.event_rx.recv().await
    }

    /// Gracefully shutdown all device tasks.
    ///
    /// Aborts all running tasks and waits for them to terminate. Collects
    /// error information from tasks but does not fail if individual tasks
    /// encounter errors. Returns an error only if shutdown itself fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_hardware::manager::PeripheralHandle;
    /// # async fn example(handle: PeripheralHandle) -> turnkey_hardware::Result<()> {
    /// handle.shutdown().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn shutdown(mut self) -> Result<()> {
        self.tasks.abort_all();

        let mut error_count = 0;
        let mut panic_count = 0;

        while let Some(result) = self.tasks.join_next().await {
            match Self::classify_task_result(result) {
                TaskTermination::Success => {}
                TaskTermination::Error => error_count += 1,
                TaskTermination::Panic => panic_count += 1,
                TaskTermination::Cancelled => {}
            }
        }

        // Future: When logging is configured, these counts could be logged
        // For now, we silently track errors but don't fail shutdown
        let _total_issues = error_count + panic_count;

        Ok(())
    }

    /// Classify the termination status of a task.
    fn classify_task_result(
        result: std::result::Result<Result<()>, tokio::task::JoinError>,
    ) -> TaskTermination {
        match result {
            Ok(Ok(())) => TaskTermination::Success,
            Ok(Err(_)) => TaskTermination::Error,
            Err(e) if e.is_cancelled() => TaskTermination::Cancelled,
            Err(_) => TaskTermination::Panic,
        }
    }
}

/// Task termination classification for shutdown handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskTermination {
    /// Task completed successfully.
    Success,
    /// Task returned an error.
    Error,
    /// Task was cancelled (expected during shutdown).
    Cancelled,
    /// Task panicked.
    Panic,
}

/// Manages all peripheral devices.
///
/// This manager coordinates multiple peripheral devices and aggregates their
/// events into a single stream for consumption by the emulator core.
///
/// # Architecture
///
/// Uses enum dispatch pattern to maintain zero-cost abstraction while providing
/// concrete type dispatch for device management. Each device runs in its own
/// async task and sends events to a shared channel.
///
/// # Lifecycle
///
/// 1. Create manager with configuration
/// 2. Register devices using `register_*` methods
/// 3. Call `start()` to spawn device tasks and get event handle
/// 4. Use handle to receive events
/// 5. Device tasks run until error or handle is dropped
///
/// # Examples
///
/// ```no_run
/// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
/// use turnkey_hardware::devices::{AnyKeypadDevice, AnyRfidDevice};
/// use turnkey_hardware::mock::{MockKeypad, MockRfid};
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     // Configure devices
///     let config = PeripheralConfig {
///         keypad_enabled: true,
///         rfid_enabled: true,
///         biometric_enabled: false,
///     };
///
///     // Create and configure manager
///     let mut manager = PeripheralManager::new(config);
///
///     let (keypad, _) = MockKeypad::new();
///     manager.register_keypad(AnyKeypadDevice::Mock(keypad));
///
///     let (rfid, _) = MockRfid::new();
///     manager.register_rfid(AnyRfidDevice::Mock(rfid));
///
///     // Start and get handle
///     let mut handle = manager.start();
///
///     // Receive events
///     while let Some(event) = handle.recv().await {
///         // Process event
///     }
///
///     Ok(())
/// }
/// ```
pub struct PeripheralManager {
    /// Registered keypad device.
    keypad: Option<AnyKeypadDevice>,

    /// Registered RFID device.
    rfid: Option<AnyRfidDevice>,

    /// Registered biometric device.
    biometric: Option<AnyBiometricDevice>,

    /// Event sender (cloned for each task).
    event_tx: mpsc::Sender<PeripheralEvent>,

    /// Event receiver (for consuming events).
    event_rx: Option<mpsc::Receiver<PeripheralEvent>>,

    /// Configuration.
    config: PeripheralConfig,
}

impl PeripheralManager {
    /// Create new peripheral manager with configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
    ///
    /// let config = PeripheralConfig::default();
    /// let manager = PeripheralManager::new(config);
    /// ```
    pub fn new(config: PeripheralConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);

        Self {
            keypad: None,
            rfid: None,
            biometric: None,
            event_tx,
            event_rx: Some(event_rx),
            config,
        }
    }

    /// Register keypad device.
    ///
    /// This must be called before `start()` if keypad is enabled in config.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
    /// use turnkey_hardware::devices::AnyKeypadDevice;
    /// use turnkey_hardware::mock::MockKeypad;
    ///
    /// let mut manager = PeripheralManager::new(PeripheralConfig::default());
    ///
    /// let (keypad, _) = MockKeypad::new();
    /// manager.register_keypad(AnyKeypadDevice::Mock(keypad));
    /// ```
    pub fn register_keypad(&mut self, device: AnyKeypadDevice) {
        self.keypad = Some(device);
    }

    /// Register RFID device.
    ///
    /// This must be called before `start()` if RFID is enabled in config.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
    /// use turnkey_hardware::devices::AnyRfidDevice;
    /// use turnkey_hardware::mock::MockRfid;
    ///
    /// let mut manager = PeripheralManager::new(PeripheralConfig::default());
    ///
    /// let (rfid, _) = MockRfid::new();
    /// manager.register_rfid(AnyRfidDevice::Mock(rfid));
    /// ```
    pub fn register_rfid(&mut self, device: AnyRfidDevice) {
        self.rfid = Some(device);
    }

    /// Register biometric device.
    ///
    /// This must be called before `start()` if biometric is enabled in config.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
    /// use turnkey_hardware::devices::AnyBiometricDevice;
    /// use turnkey_hardware::mock::MockBiometric;
    ///
    /// let config = PeripheralConfig {
    ///     keypad_enabled: false,
    ///     rfid_enabled: false,
    ///     biometric_enabled: true,
    /// };
    ///
    /// let mut manager = PeripheralManager::new(config);
    ///
    /// let (biometric, _) = MockBiometric::new();
    /// manager.register_biometric(AnyBiometricDevice::Mock(biometric));
    /// ```
    pub fn register_biometric(&mut self, device: AnyBiometricDevice) {
        self.biometric = Some(device);
    }

    /// Start listening to all devices and return event handle.
    ///
    /// Spawns async tasks for each enabled device. Each task runs independently
    /// and sends events to the unified event channel. Returns a handle that can
    /// be used to receive events from all devices.
    ///
    /// This method consumes `self` and returns a `PeripheralHandle` that provides
    /// access to the event stream and allows graceful shutdown.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
    /// use turnkey_hardware::devices::AnyKeypadDevice;
    /// use turnkey_hardware::mock::MockKeypad;
    ///
    /// #[tokio::main]
    /// async fn main() -> turnkey_hardware::Result<()> {
    ///     let mut manager = PeripheralManager::new(PeripheralConfig::default());
    ///
    ///     let (keypad, _) = MockKeypad::new();
    ///     manager.register_keypad(AnyKeypadDevice::Mock(keypad));
    ///
    ///     let mut handle = manager.start();
    ///
    ///     // Receive events
    ///     while let Some(event) = handle.recv().await {
    ///         // Process event
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn start(mut self) -> PeripheralHandle {
        let mut tasks = JoinSet::new();

        // Spawn keypad task
        if self.config.keypad_enabled
            && let Some(device) = self.keypad.take()
        {
            let tx = self.event_tx.clone();
            tasks.spawn(Self::keypad_task(device, tx));
        }

        // Spawn RFID task
        if self.config.rfid_enabled
            && let Some(device) = self.rfid.take()
        {
            let tx = self.event_tx.clone();
            tasks.spawn(Self::rfid_task(device, tx));
        }

        // Spawn biometric task
        if self.config.biometric_enabled
            && let Some(device) = self.biometric.take()
        {
            let tx = self.event_tx.clone();
            tasks.spawn(Self::biometric_task(device, tx));
        }

        PeripheralHandle {
            event_rx: self.event_rx.take().expect("Event receiver already taken"),
            tasks,
        }
    }

    /// Check if specific device type is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig, DeviceType};
    ///
    /// let manager = PeripheralManager::new(PeripheralConfig::default());
    ///
    /// assert!(manager.is_device_enabled(DeviceType::Keypad));
    /// assert!(manager.is_device_enabled(DeviceType::Rfid));
    /// ```
    pub fn is_device_enabled(&self, device_type: DeviceType) -> bool {
        match device_type {
            DeviceType::Keypad => self.config.keypad_enabled,
            DeviceType::Rfid => self.config.rfid_enabled,
            DeviceType::Biometric => self.config.biometric_enabled,
        }
    }

    /// Get device statistics.
    ///
    /// Returns information about which devices are currently registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::manager::{PeripheralManager, PeripheralConfig};
    /// use turnkey_hardware::devices::AnyKeypadDevice;
    /// use turnkey_hardware::mock::MockKeypad;
    ///
    /// let mut manager = PeripheralManager::new(PeripheralConfig::default());
    ///
    /// let (keypad, _) = MockKeypad::new();
    /// manager.register_keypad(AnyKeypadDevice::Mock(keypad));
    ///
    /// let stats = manager.get_stats();
    /// assert!(stats.keypad_connected);
    /// assert!(!stats.rfid_connected);
    /// ```
    pub fn get_stats(&self) -> PeripheralStats {
        PeripheralStats {
            keypad_connected: self.keypad.is_some(),
            rfid_connected: self.rfid.is_some(),
            biometric_connected: self.biometric.is_some(),
        }
    }

    // Private task functions

    async fn keypad_task(
        mut device: AnyKeypadDevice,
        tx: mpsc::Sender<PeripheralEvent>,
    ) -> Result<()> {
        // Rate limiting: minimum delay between polls to prevent busy-waiting
        const MIN_POLL_INTERVAL_MS: u64 = 10; // 100 Hz maximum

        loop {
            let start = tokio::time::Instant::now();

            match device.read_input().await {
                Ok(input) => {
                    // Use try_send to detect backpressure
                    match tx.try_send(PeripheralEvent::KeypadInput(input)) {
                        Ok(_) => {}
                        Err(tokio::sync::mpsc::error::TrySendError::Full(event)) => {
                            // Channel is full - apply backpressure
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            // Retry once with blocking send
                            if tx.send(event).await.is_err() {
                                break; // Channel closed
                            }
                        }
                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                            break; // Channel closed
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(PeripheralEvent::DeviceError {
                            device_type: DeviceType::Keypad,
                            error: e.to_string(),
                        })
                        .await;
                    return Err(e);
                }
            }

            // Rate limiting to prevent excessive CPU usage
            let elapsed = start.elapsed();
            if elapsed.as_millis() < MIN_POLL_INTERVAL_MS as u128 {
                tokio::time::sleep(
                    tokio::time::Duration::from_millis(MIN_POLL_INTERVAL_MS) - elapsed,
                )
                .await;
            }
        }
        Ok(())
    }

    async fn rfid_task(mut device: AnyRfidDevice, tx: mpsc::Sender<PeripheralEvent>) -> Result<()> {
        // Rate limiting: minimum delay between polls to prevent busy-waiting
        const MIN_POLL_INTERVAL_MS: u64 = 10; // 100 Hz maximum

        loop {
            let start = tokio::time::Instant::now();

            match device.read_card().await {
                Ok(card) => {
                    // Use try_send to detect backpressure
                    match tx.try_send(PeripheralEvent::CardRead(card)) {
                        Ok(_) => {}
                        Err(tokio::sync::mpsc::error::TrySendError::Full(event)) => {
                            // Channel is full - apply backpressure
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            // Retry once with blocking send
                            if tx.send(event).await.is_err() {
                                break; // Channel closed
                            }
                        }
                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                            break; // Channel closed
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(PeripheralEvent::DeviceError {
                            device_type: DeviceType::Rfid,
                            error: e.to_string(),
                        })
                        .await;
                    return Err(e);
                }
            }

            // Rate limiting to prevent excessive CPU usage
            let elapsed = start.elapsed();
            if elapsed.as_millis() < MIN_POLL_INTERVAL_MS as u128 {
                tokio::time::sleep(
                    tokio::time::Duration::from_millis(MIN_POLL_INTERVAL_MS) - elapsed,
                )
                .await;
            }
        }
        Ok(())
    }

    async fn biometric_task(
        mut device: AnyBiometricDevice,
        tx: mpsc::Sender<PeripheralEvent>,
    ) -> Result<()> {
        // Rate limiting: minimum delay between polls to prevent busy-waiting
        const MIN_POLL_INTERVAL_MS: u64 = 10; // 100 Hz maximum

        loop {
            let start = tokio::time::Instant::now();

            match device.capture_fingerprint().await {
                Ok(data) => {
                    // Use try_send to detect backpressure
                    match tx.try_send(PeripheralEvent::FingerprintCaptured(data)) {
                        Ok(_) => {}
                        Err(tokio::sync::mpsc::error::TrySendError::Full(event)) => {
                            // Channel is full - apply backpressure
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            // Retry once with blocking send
                            if tx.send(event).await.is_err() {
                                break; // Channel closed
                            }
                        }
                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                            break; // Channel closed
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(PeripheralEvent::DeviceError {
                            device_type: DeviceType::Biometric,
                            error: e.to_string(),
                        })
                        .await;
                    return Err(e);
                }
            }

            // Rate limiting to prevent excessive CPU usage
            let elapsed = start.elapsed();
            if elapsed.as_millis() < MIN_POLL_INTERVAL_MS as u128 {
                tokio::time::sleep(
                    tokio::time::Duration::from_millis(MIN_POLL_INTERVAL_MS) - elapsed,
                )
                .await;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::{AnyBiometricDevice, AnyKeypadDevice, AnyRfidDevice};
    use crate::traits::CardType;

    #[test]
    fn test_peripheral_config_default() {
        let config = PeripheralConfig::default();
        assert!(config.keypad_enabled);
        assert!(config.rfid_enabled);
        assert!(!config.biometric_enabled);
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Keypad.to_string(), "Keypad");
        assert_eq!(DeviceType::Rfid.to_string(), "RFID");
        assert_eq!(DeviceType::Biometric.to_string(), "Biometric");
    }

    #[test]
    fn test_manager_new() {
        let config = PeripheralConfig::default();
        let manager = PeripheralManager::new(config.clone());

        assert_eq!(manager.config, config);
        assert!(manager.keypad.is_none());
        assert!(manager.rfid.is_none());
        assert!(manager.biometric.is_none());
    }

    #[test]
    fn test_manager_register_devices() {
        let mut manager = PeripheralManager::new(PeripheralConfig::default());

        let (keypad, _) = crate::mock::MockKeypad::new();
        manager.register_keypad(AnyKeypadDevice::Mock(keypad));

        let (rfid, _) = crate::mock::MockRfid::new();
        manager.register_rfid(AnyRfidDevice::Mock(rfid));

        let (biometric, _) = crate::mock::MockBiometric::new();
        manager.register_biometric(AnyBiometricDevice::Mock(biometric));

        assert!(manager.keypad.is_some());
        assert!(manager.rfid.is_some());
        assert!(manager.biometric.is_some());
    }

    #[test]
    fn test_manager_get_stats() {
        let mut manager = PeripheralManager::new(PeripheralConfig::default());

        let stats = manager.get_stats();
        assert!(!stats.keypad_connected);
        assert!(!stats.rfid_connected);
        assert!(!stats.biometric_connected);

        let (keypad, _) = crate::mock::MockKeypad::new();
        manager.register_keypad(AnyKeypadDevice::Mock(keypad));

        let stats = manager.get_stats();
        assert!(stats.keypad_connected);
        assert!(!stats.rfid_connected);
        assert!(!stats.biometric_connected);
    }

    #[test]
    fn test_manager_is_device_enabled() {
        let config = PeripheralConfig {
            keypad_enabled: true,
            rfid_enabled: false,
            biometric_enabled: true,
        };

        let manager = PeripheralManager::new(config);

        assert!(manager.is_device_enabled(DeviceType::Keypad));
        assert!(!manager.is_device_enabled(DeviceType::Rfid));
        assert!(manager.is_device_enabled(DeviceType::Biometric));
    }

    #[tokio::test]
    async fn test_manager_keypad_events() {
        // NOTE: Current API design limitation - start() consumes self and recv_event()
        // requires &mut self, making it impossible to test event reception in the
        // current design. This test verifies manager can start successfully.
        //
        // TODO: Consider refactoring to split concerns:
        // - start() should return a handle that can be used to recv events
        // - Or provide a separate recv() method that works with a shared manager

        let mut manager = PeripheralManager::new(PeripheralConfig {
            keypad_enabled: true,
            rfid_enabled: false,
            biometric_enabled: false,
        });

        let (keypad, _handle) = crate::mock::MockKeypad::new();
        manager.register_keypad(AnyKeypadDevice::Mock(keypad));

        // Verify manager is configured correctly before starting
        let stats = manager.get_stats();
        assert!(stats.keypad_connected);
        assert!(!stats.rfid_connected);
        assert!(!stats.biometric_connected);

        // Start manager and get handle
        let handle = manager.start();

        // Give manager time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Shutdown the handle
        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_manager_rfid_events() {
        // Similar to keypad test - verifies RFID configuration and startup
        let config = PeripheralConfig {
            keypad_enabled: false,
            rfid_enabled: true,
            biometric_enabled: false,
        };

        let mut manager = PeripheralManager::new(config);

        let (rfid, mut handle) = crate::mock::MockRfid::new();
        manager.register_rfid(AnyRfidDevice::Mock(rfid));

        // Verify configuration
        let stats = manager.get_stats();
        assert!(!stats.keypad_connected);
        assert!(stats.rfid_connected);
        assert!(!stats.biometric_connected);

        // Prepare mock card for reading
        let uid = vec![0x01, 0x02, 0x03, 0x04];
        handle
            .add_card(uid.clone(), CardType::MifareClassic1K)
            .await;
        assert!(handle.card_count() > 0);

        // Start manager and get handle
        let manager_handle = manager.start();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Shutdown the handle
        manager_handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_manager_multiple_devices() {
        // Test manager can handle multiple device types concurrently
        let config = PeripheralConfig {
            keypad_enabled: true,
            rfid_enabled: true,
            biometric_enabled: true,
        };

        let mut manager = PeripheralManager::new(config);

        let (keypad, _keypad_handle) = crate::mock::MockKeypad::new();
        manager.register_keypad(AnyKeypadDevice::Mock(keypad));

        let (rfid, _rfid_handle) = crate::mock::MockRfid::new();
        manager.register_rfid(AnyRfidDevice::Mock(rfid));

        let (biometric, _bio_handle) = crate::mock::MockBiometric::new();
        manager.register_biometric(AnyBiometricDevice::Mock(biometric));

        // Verify all devices registered
        let stats = manager.get_stats();
        assert!(stats.keypad_connected);
        assert!(stats.rfid_connected);
        assert!(stats.biometric_connected);

        // Start manager with all devices
        let handle = manager.start();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Shutdown the handle
        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_manager_graceful_shutdown() {
        let config = PeripheralConfig::default();
        let manager = PeripheralManager::new(config);

        // Test shutdown of manager with no devices
        let handle = manager.start();
        handle.shutdown().await.unwrap();
    }
}
