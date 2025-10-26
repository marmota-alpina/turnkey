//! Mock RFID reader implementation for testing and development.
//!
//! This module provides a simulated RFID reader that can be controlled
//! programmatically for testing without requiring physical hardware.

use crate::{
    Result,
    traits::{CardData, CardType, RfidDevice},
    types::{LedColor, ReaderInfo},
};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Mock RFID reader for testing and development.
///
/// This device simulates an RFID/NFC card reader by maintaining a database
/// of cards that can be programmatically presented to the reader.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::mock::MockRfid;
/// use turnkey_hardware::traits::{RfidDevice, CardType};
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (mut reader, mut handle) = MockRfid::new();
///
///     // Register a card
///     let card_uid = vec![0x04, 0xAB, 0xCD, 0xEF];
///     handle.add_card(card_uid.clone(), CardType::MifareClassic1K).await;
///
///     // Present the card
///     handle.present_card(card_uid).await?;
///
///     // Read the card
///     let card = reader.read_card().await?;
///     assert_eq!(card.uid_hex(), "04ABCDEF");
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct MockRfid {
    /// Channel receiver for card events
    event_rx: mpsc::Receiver<CardEvent>,

    /// Device name
    name: String,

    /// Currently set LED color
    led_color: LedColor,
}

impl MockRfid {
    /// Create a new mock RFID reader with the default name.
    ///
    /// Returns a tuple of (MockRfid, MockRfidHandle) where the handle
    /// can be used to simulate card presentations.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockRfid;
    ///
    /// let (reader, handle) = MockRfid::new();
    /// ```
    pub fn new() -> (Self, MockRfidHandle) {
        Self::with_name("Mock RFID Reader".to_string())
    }

    /// Create a new mock RFID reader with a custom name.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockRfid;
    ///
    /// let (reader, handle) = MockRfid::with_name("Test Reader 1".to_string());
    /// ```
    pub fn with_name(name: String) -> (Self, MockRfidHandle) {
        let (event_tx, event_rx) = mpsc::channel(32);

        let reader = Self {
            event_rx,
            name: name.clone(),
            led_color: LedColor::Off,
        };

        let handle = MockRfidHandle {
            event_tx,
            name,
            cards: HashMap::new(),
            current_card: None,
        };

        (reader, handle)
    }

    /// Get the current LED color.
    ///
    /// This is useful for testing LED control.
    pub fn led_color(&self) -> LedColor {
        self.led_color
    }
}

impl Default for MockRfid {
    fn default() -> Self {
        Self::new().0
    }
}

impl RfidDevice for MockRfid {
    async fn read_card(&mut self) -> Result<CardData> {
        let event = self
            .event_rx
            .recv()
            .await
            .ok_or_else(|| crate::HardwareError::disconnected("RFID event channel closed"))?;

        match event {
            CardEvent::CardPresented(card) => Ok(card),
        }
    }

    async fn is_card_present(&self) -> Result<bool> {
        // In mock implementation, check if channel has pending events
        // This is a best-effort check and may not be 100% accurate
        Ok(!self.event_rx.is_empty())
    }

    async fn get_reader_info(&self) -> Result<ReaderInfo> {
        Ok(ReaderInfo::new(
            self.name.clone(),
            vec!["ISO14443A".to_string(), "ISO14443B".to_string()],
        )
        .with_max_baud_rate(424000))
    }

    async fn set_led(&mut self, color: LedColor) -> Result<()> {
        self.led_color = color;
        Ok(())
    }
}

/// Internal event type for mock RFID reader.
#[derive(Debug, Clone)]
enum CardEvent {
    CardPresented(CardData),
}

/// Handle for controlling a mock RFID reader.
///
/// This handle allows programmatic control of the mock reader by managing
/// a card database and simulating card presentations.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::mock::MockRfid;
/// use turnkey_hardware::traits::CardType;
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (_reader, mut handle) = MockRfid::new();
///
///     // Add cards to the database
///     let card1 = vec![0x01, 0x02, 0x03, 0x04];
///     let card2 = vec![0x05, 0x06, 0x07, 0x08];
///
///     handle.add_card(card1.clone(), CardType::MifareClassic1K).await;
///     handle.add_card(card2.clone(), CardType::MifareClassic4K).await;
///
///     // Present cards
///     handle.present_card(card1).await?;
///     handle.present_card(card2).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MockRfidHandle {
    /// Channel sender for card events
    event_tx: mpsc::Sender<CardEvent>,

    /// Device name
    name: String,

    /// Card database (UID -> CardType)
    cards: HashMap<Vec<u8>, CardType>,

    /// Currently presented card
    current_card: Option<Vec<u8>>,
}

impl MockRfidHandle {
    /// Add a card to the reader's database.
    ///
    /// This registers a card that can be presented to the reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockRfid;
    /// use turnkey_hardware::traits::CardType;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (_reader, mut handle) = MockRfid::new();
    ///
    ///     let uid = vec![0x04, 0xAB, 0xCD, 0xEF];
    ///     handle.add_card(uid, CardType::MifareClassic1K).await;
    /// }
    /// ```
    pub async fn add_card(&mut self, uid: Vec<u8>, card_type: CardType) {
        self.cards.insert(uid, card_type);
    }

    /// Present a card to the reader.
    ///
    /// The card must have been previously added with `add_card()`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The card UID is not in the database
    /// - The reader has been dropped and the channel is closed
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockRfid;
    /// use turnkey_hardware::traits::CardType;
    ///
    /// #[tokio::main]
    /// async fn main() -> turnkey_hardware::Result<()> {
    ///     let (_reader, mut handle) = MockRfid::new();
    ///
    ///     let uid = vec![0x04, 0xAB, 0xCD, 0xEF];
    ///     handle.add_card(uid.clone(), CardType::MifareClassic1K).await;
    ///     handle.present_card(uid).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn present_card(&mut self, uid: Vec<u8>) -> Result<()> {
        let card_type = self
            .cards
            .get(&uid)
            .ok_or_else(|| {
                crate::HardwareError::invalid_data(format!("Card {:02X?} not in database", uid))
            })?
            .clone();

        let card = CardData::new(uid.clone(), card_type)?;
        self.current_card = Some(uid);

        self.event_tx
            .send(CardEvent::CardPresented(card))
            .await
            .map_err(|_| crate::HardwareError::disconnected("RFID event channel closed"))?;

        Ok(())
    }

    /// Remove the current card from the reader.
    ///
    /// This simulates the card being removed from the reader's field.
    pub fn remove_card(&mut self) {
        self.current_card = None;
    }

    /// Check if a card is currently presented.
    pub fn is_card_presented(&self) -> bool {
        self.current_card.is_some()
    }

    /// Get the UID of the currently presented card, if any.
    pub fn current_card_uid(&self) -> Option<&[u8]> {
        self.current_card.as_deref()
    }

    /// Get the device name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of cards in the database.
    pub fn card_count(&self) -> usize {
        self.cards.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_rfid_present_and_read() {
        let (mut reader, mut handle) = MockRfid::new();

        let uid = vec![0x04, 0xAB, 0xCD, 0xEF];
        handle
            .add_card(uid.clone(), CardType::MifareClassic1K)
            .await;

        tokio::spawn(async move {
            handle.present_card(uid).await.unwrap();
        });

        let card = reader.read_card().await.unwrap();
        assert_eq!(card.uid_hex(), "04ABCDEF");
        assert_eq!(card.card_type, CardType::MifareClassic1K);
    }

    #[tokio::test]
    async fn test_mock_rfid_multiple_cards() {
        let (mut reader, mut handle) = MockRfid::new();

        let card1 = vec![0x01, 0x02, 0x03, 0x04];
        let card2 = vec![0x05, 0x06, 0x07, 0x08];

        handle
            .add_card(card1.clone(), CardType::MifareClassic1K)
            .await;
        handle
            .add_card(card2.clone(), CardType::MifareClassic4K)
            .await;

        assert_eq!(handle.card_count(), 2);

        tokio::spawn(async move {
            handle.present_card(card1).await.unwrap();
            handle.present_card(card2).await.unwrap();
        });

        let read1 = reader.read_card().await.unwrap();
        assert_eq!(read1.card_type, CardType::MifareClassic1K);

        let read2 = reader.read_card().await.unwrap();
        assert_eq!(read2.card_type, CardType::MifareClassic4K);
    }

    #[tokio::test]
    async fn test_mock_rfid_unknown_card() {
        let (_reader, mut handle) = MockRfid::new();

        let unknown_uid = vec![0xFF, 0xFF, 0xFF, 0xFF];

        let result = handle.present_card(unknown_uid).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_rfid_led_control() {
        let (mut reader, _handle) = MockRfid::new();

        assert_eq!(reader.led_color(), LedColor::Off);

        reader.set_led(LedColor::Green).await.unwrap();
        assert_eq!(reader.led_color(), LedColor::Green);

        reader.set_led(LedColor::Red).await.unwrap();
        assert_eq!(reader.led_color(), LedColor::Red);
    }

    #[tokio::test]
    async fn test_mock_rfid_get_reader_info() {
        let (reader, _handle) = MockRfid::with_name("Test Reader".to_string());

        let info = reader.get_reader_info().await.unwrap();
        assert_eq!(info.name, "Test Reader");
        assert!(info.protocols.contains(&"ISO14443A".to_string()));
        assert_eq!(info.max_baud_rate, Some(424000));
    }

    #[tokio::test]
    async fn test_mock_rfid_handle_remove_card() {
        let (_reader, mut handle) = MockRfid::new();

        let uid = vec![0x01, 0x02, 0x03, 0x04];
        handle
            .add_card(uid.clone(), CardType::MifareClassic1K)
            .await;

        handle.present_card(uid.clone()).await.unwrap();
        assert!(handle.is_card_presented());
        assert_eq!(handle.current_card_uid(), Some(uid.as_slice()));

        handle.remove_card();
        assert!(!handle.is_card_presented());
        assert_eq!(handle.current_card_uid(), None);
    }

    #[tokio::test]
    async fn test_mock_rfid_handle_clone() {
        let (_reader, mut handle) = MockRfid::new();

        let uid1 = vec![0x01, 0x02, 0x03, 0x04];
        let uid2 = vec![0x05, 0x06, 0x07, 0x08];

        handle
            .add_card(uid1.clone(), CardType::MifareClassic1K)
            .await;

        let mut handle_clone = handle.clone();
        handle_clone
            .add_card(uid2.clone(), CardType::MifareClassic4K)
            .await;

        // Each handle has independent card database after clone
        assert_eq!(handle.card_count(), 1); // Only uid1
        assert_eq!(handle_clone.card_count(), 2); // uid1 + uid2
    }

    #[tokio::test]
    async fn test_mock_rfid_card_uid_decimal() {
        let (mut reader, mut handle) = MockRfid::new();

        let uid = vec![0x01, 0x02, 0x03, 0x04];
        handle
            .add_card(uid.clone(), CardType::MifareClassic1K)
            .await;

        tokio::spawn(async move {
            handle.present_card(uid).await.unwrap();
        });

        let card = reader.read_card().await.unwrap();
        assert_eq!(card.uid_decimal(), "16909060");
    }
}
