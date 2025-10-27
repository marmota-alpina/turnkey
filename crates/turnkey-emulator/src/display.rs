//! Virtual LCD display implementation for turnstile emulation.
//!
//! This module provides a virtual 2-line × 40-column LCD display that simulates
//! the physical turnstile display. It handles text rendering, alignment, temporary
//! messages with timeouts, and automatic state machine integration.
//!
//! # Character Encoding - ASCII Only
//!
//! **CRITICAL**: This display only accepts ASCII characters (0x20-0x7E).
//!
//! The Henry protocol specification mandates ASCII-only encoding for all display
//! messages. Physical turnstile hardware LCD displays do not support extended
//! character sets or Unicode.
//!
//! ## Why ASCII Only?
//!
//! This emulator intentionally rejects non-ASCII input to ensure developers
//! test their integrations with the same constraints as real hardware. If the
//! emulator accepted UTF-8 and performed automatic transliteration, integration
//! issues would only surface when connecting to physical equipment in production.
//!
//! ## Handling Portuguese Text
//!
//! If your application needs to display Portuguese messages with accents,
//! you must transliterate them to ASCII **before** sending to the display.
//! This matches the real-world requirement where the server/application is
//! responsible for ASCII conversion before transmitting to the turnstile.
//!
//! Example:
//! ```rust,ignore
//! // In your application code (not in the emulator):
//! fn to_ascii(text: &str) -> String {
//!     text.replace("ã", "a")
//!         .replace("ç", "c")
//!         .replace("é", "e")
//!         // ... other replacements
//! }
//!
//! let message = to_ascii("Liberação concedida");
//! display.set_line(0, &message).unwrap();
//! ```
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```
//! use turnkey_emulator::VirtualDisplay;
//!
//! let mut display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
//! display.set_line(0, "AGUARDE...").unwrap();
//! display.set_line(1, "Lendo credencial").unwrap();
//!
//! assert_eq!(display.get_line(0).unwrap(), "AGUARDE...                              ");
//! ```
//!
//! ## Temporary Messages
//!
//! ```
//! use std::time::Duration;
//! use turnkey_emulator::VirtualDisplay;
//!
//! let mut display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
//! display.show_temporary("ACESSO LIBERADO", Duration::from_secs(5)).unwrap();
//!
//! // Message will auto-clear after 5 seconds when update() is called
//! assert!(!display.is_default());
//! ```
//!
//! ## State Machine Integration
//!
//! ```
//! use turnkey_emulator::{VirtualDisplay, TurnstileState};
//!
//! let mut display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
//! display.update_from_state(&TurnstileState::Validating);
//!
//! assert_eq!(display.get_line(0).unwrap().trim(), "VALIDANDO...");
//! ```
//!
//! ## Builder Pattern
//!
//! ```
//! use turnkey_emulator::VirtualDisplay;
//!
//! let display = VirtualDisplay::builder()
//!     .with_size(2, 40)
//!     .with_default_message("WELCOME".to_string())
//!     .build();
//!
//! assert_eq!(display.get_line(0).unwrap().trim(), "WELCOME");
//! ```

use std::time::{Duration, Instant};

use turnkey_core::{Error, Result};

use crate::TurnstileState;

/// Maximum number of display lines (standard LCD configuration).
const DEFAULT_LINES: usize = 2;

/// Maximum number of characters per line (standard LCD configuration).
const DEFAULT_COLUMNS: usize = 40;

/// Text alignment options for display lines.
///
/// These options control how text is positioned within the 40-character line width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Text starts at column 0, padded with spaces on the right.
    Left,
    /// Text centered with equal padding on both sides (extra space on right if odd).
    Center,
    /// Text ends at column 40, padded with spaces on the left.
    Right,
}

/// Virtual LCD display for turnstile emulation.
///
/// This struct manages a 2-line × 40-column virtual display, providing text
/// rendering, alignment, temporary messages with timeouts, and state machine
/// integration.
///
/// # Thread Safety
///
/// This struct is not thread-safe by design. In async contexts, protect
/// access using `tokio::sync::Mutex` or similar synchronization primitive.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::VirtualDisplay;
///
/// let mut display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
///
/// display.set_line(0, "AGUARDE...").unwrap();
/// display.set_line(1, "Lendo credencial").unwrap();
///
/// assert_eq!(display.get_all_lines().len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct VirtualDisplay {
    /// Number of lines in the display.
    lines: usize,

    /// Number of columns per line.
    columns: usize,

    /// Current display buffer (ASCII characters only).
    buffer: Vec<String>,

    /// Default message to show when idle.
    default_message: String,

    /// Temporary message with expiration timestamp.
    temporary_message: Option<(String, Instant)>,
}

impl VirtualDisplay {
    /// Create a new virtual display with specified dimensions and default message.
    ///
    /// # Arguments
    ///
    /// * `lines` - Number of lines (typically 2)
    /// * `columns` - Number of columns per line (typically 40)
    /// * `default_message` - Default message shown when idle (ASCII only)
    ///
    /// # Returns
    ///
    /// Returns a new `VirtualDisplay` initialized with the default message
    /// on the first line, centered.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `default_message` contains non-ASCII characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
    /// assert_eq!(display.get_line(0).unwrap().trim(), "DIGITE SEU CODIGO");
    /// ```
    pub fn new(lines: usize, columns: usize, default_message: String) -> Self {
        debug_assert!(
            default_message.is_ascii(),
            "Default message must be ASCII only. Got: '{}'",
            default_message
        );

        let mut buffer = vec![" ".repeat(columns); lines];

        // Set default message on first line, centered
        if !default_message.is_empty() {
            buffer[0] = align_text(&default_message, columns, Alignment::Center);
        }

        Self {
            lines,
            columns,
            buffer,
            default_message,
            temporary_message: None,
        }
    }

    /// Create a builder for constructing a virtual display with custom configuration.
    ///
    /// # Returns
    ///
    /// Returns a new `VirtualDisplayBuilder` for fluent configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let display = VirtualDisplay::builder()
    ///     .with_size(2, 40)
    ///     .with_default_message("WELCOME".to_string())
    ///     .build();
    /// ```
    pub fn builder() -> VirtualDisplayBuilder {
        VirtualDisplayBuilder::default()
    }

    /// Set text on a specific line with left alignment.
    ///
    /// Control characters are removed, and text is truncated to fit within
    /// the column width.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index (0-based)
    /// * `text` - Text to display (ASCII only)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Line index is out of bounds
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `text` contains non-ASCII characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
    /// display.set_line(0, "AGUARDE...").unwrap();
    /// assert_eq!(display.get_line(0).unwrap().trim_end(), "AGUARDE...");
    /// ```
    pub fn set_line(&mut self, line: usize, text: &str) -> Result<()> {
        self.set_line_aligned(line, text, Alignment::Left)
    }

    /// Set text on a specific line with custom alignment.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index (0-based)
    /// * `text` - Text to display
    /// * `align` - Text alignment (Left, Center, or Right)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Line index is out of bounds
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::{VirtualDisplay, Alignment};
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
    /// display.set_line_aligned(0, "CENTERED", Alignment::Center).unwrap();
    /// ```
    pub fn set_line_aligned(&mut self, line: usize, text: &str, align: Alignment) -> Result<()> {
        debug_assert!(
            text.is_ascii(),
            "Display text must be ASCII only for protocol compatibility. Got: '{}'",
            text
        );

        if line >= self.lines {
            return Err(Error::InvalidLine {
                line,
                max: self.lines - 1,
            });
        }

        let sanitized = sanitize_text(text);
        let aligned = align_text(&sanitized, self.columns, align);

        self.buffer[line] = aligned;
        Ok(())
    }

    /// Set both lines simultaneously.
    ///
    /// Both lines are set with left alignment. This is a convenience method
    /// for setting multiple lines at once.
    ///
    /// # Arguments
    ///
    /// * `line1` - Text for first line
    /// * `line2` - Text for second line
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Display has fewer than 2 lines
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
    /// display.set_lines("AGUARDE...", "Validando").unwrap();
    /// ```
    pub fn set_lines(&mut self, line1: &str, line2: &str) -> Result<()> {
        self.set_line(0, line1)?;
        self.set_line(1, line2)?;
        Ok(())
    }

    /// Show a temporary message that auto-clears after the specified duration.
    ///
    /// The message is displayed on the first line, centered. When `update()` is
    /// called and the duration has elapsed, the display automatically returns
    /// to the default message.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to display temporarily
    /// * `duration` - How long to show the message
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Duration is zero
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
    /// display.show_temporary("ACESSO LIBERADO", Duration::from_secs(5)).unwrap();
    /// assert!(!display.is_default());
    /// ```
    pub fn show_temporary(&mut self, text: &str, duration: Duration) -> Result<()> {
        debug_assert!(
            text.is_ascii(),
            "Display text must be ASCII only for protocol compatibility. Got: '{}'",
            text
        );

        if duration.is_zero() {
            return Err(Error::InvalidDuration);
        }

        let expiration = Instant::now() + duration;
        let sanitized = sanitize_text(text);

        self.temporary_message = Some((sanitized.clone(), expiration));
        self.set_line_aligned(0, &sanitized, Alignment::Center)?;
        self.set_line(1, "")?;

        Ok(())
    }

    /// Update display state, checking for expired temporary messages.
    ///
    /// This method should be called periodically in your event loop to handle
    /// automatic expiration of temporary messages.
    ///
    /// # Returns
    ///
    /// Returns `true` if the display content changed, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
    /// display.show_temporary("MESSAGE", Duration::from_secs(1)).unwrap();
    ///
    /// // In your event loop:
    /// loop {
    ///     if display.update() {
    ///         // Display content changed, redraw if needed
    ///     }
    ///     std::thread::sleep(Duration::from_millis(100));
    /// }
    /// ```
    pub fn update(&mut self) -> bool {
        if let Some((_, expiration)) = self.temporary_message
            && Instant::now() >= expiration
        {
            self.temporary_message = None;
            self.reset_to_default();
            return true;
        }
        false
    }

    /// Clear all lines by filling them with spaces.
    ///
    /// This does not reset to the default message; use `reset_to_default()`
    /// for that purpose.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
    /// display.set_line(0, "TEXT").unwrap();
    /// display.clear();
    /// assert_eq!(display.get_line(0).unwrap().trim(), "");
    /// ```
    pub fn clear(&mut self) {
        for line in &mut self.buffer {
            *line = " ".repeat(self.columns);
        }
        self.temporary_message = None;
    }

    /// Reset display to show the default message.
    ///
    /// The default message is shown on the first line, centered, and all
    /// other lines are cleared. This also clears any temporary message.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
    /// display.set_line(0, "TEMPORARY").unwrap();
    /// display.reset_to_default();
    /// assert!(display.is_default());
    /// ```
    pub fn reset_to_default(&mut self) {
        self.clear();
        self.temporary_message = None;

        if !self.default_message.is_empty() {
            self.buffer[0] = align_text(&self.default_message, self.columns, Alignment::Center);
        }
    }

    /// Update display based on state machine state.
    ///
    /// This method automatically sets appropriate messages for each state,
    /// all using ASCII-compatible text for protocol compliance.
    ///
    /// # Arguments
    ///
    /// * `state` - Current turnstile state
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::{VirtualDisplay, TurnstileState};
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
    /// display.update_from_state(&TurnstileState::Granted);
    /// assert_eq!(display.get_line(0).unwrap().trim(), "ACESSO LIBERADO");
    /// ```
    pub fn update_from_state(&mut self, state: &TurnstileState) {
        let (line1, line2) = match state {
            TurnstileState::Idle => (self.default_message.clone(), String::new()),
            TurnstileState::Reading => ("AGUARDE...".into(), "Lendo credencial".into()),
            TurnstileState::Validating => ("VALIDANDO...".into(), "Aguarde resposta".into()),
            TurnstileState::Granted => ("ACESSO LIBERADO".into(), String::new()),
            TurnstileState::Denied => ("ACESSO NEGADO".into(), String::new()),
            TurnstileState::WaitingRotation => ("PASSE PELA CATRACA".into(), String::new()),
            TurnstileState::RotationInProgress => ("GIRANDO...".into(), String::new()),
            TurnstileState::RotationCompleted => ("OBRIGADO".into(), String::new()),
            TurnstileState::RotationTimeout => ("TEMPO ESGOTADO".into(), String::new()),
        };

        // All messages are already ASCII-compatible or will be transliterated
        let _ = self.set_line_aligned(0, &line1, Alignment::Center);
        let _ = self.set_line_aligned(1, &line2, Alignment::Center);
    }

    /// Get text from a specific line.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index (0-based)
    ///
    /// # Returns
    ///
    /// Returns the text from the specified line, padded to column width.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Line index is out of bounds
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let display = VirtualDisplay::new(2, 40, "HELLO".to_string());
    /// assert_eq!(display.get_line(0).unwrap().len(), 40);
    /// ```
    pub fn get_line(&self, line: usize) -> Result<&str> {
        if line >= self.lines {
            return Err(Error::InvalidLine {
                line,
                max: self.lines - 1,
            });
        }
        Ok(&self.buffer[line])
    }

    /// Get all lines as a vector.
    ///
    /// # Returns
    ///
    /// Returns a vector of string slices, one for each line.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let display = VirtualDisplay::new(2, 40, "HELLO".to_string());
    /// let lines = display.get_all_lines();
    /// assert_eq!(lines.len(), 2);
    /// ```
    pub fn get_all_lines(&self) -> Vec<&str> {
        self.buffer.iter().map(|s| s.as_str()).collect()
    }

    /// Check if display is showing the default message.
    ///
    /// # Returns
    ///
    /// Returns `true` if the first line contains the default message (ignoring
    /// padding/alignment) and no temporary message is active.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::VirtualDisplay;
    ///
    /// let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
    /// assert!(display.is_default());
    ///
    /// display.set_line(0, "OTHER").unwrap();
    /// assert!(!display.is_default());
    /// ```
    pub fn is_default(&self) -> bool {
        if self.temporary_message.is_some() {
            return false;
        }

        let first_line_trimmed = self.buffer[0].trim();
        first_line_trimmed == self.default_message
    }
}

/// Builder for constructing `VirtualDisplay` instances with custom configuration.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::VirtualDisplay;
///
/// let display = VirtualDisplay::builder()
///     .with_size(2, 40)
///     .with_default_message("WELCOME".to_string())
///     .build();
/// ```
#[derive(Debug)]
pub struct VirtualDisplayBuilder {
    lines: usize,
    columns: usize,
    default_message: String,
}

impl VirtualDisplayBuilder {
    /// Set the display size (lines and columns).
    ///
    /// # Arguments
    ///
    /// * `lines` - Number of lines
    /// * `columns` - Number of columns per line
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn with_size(mut self, lines: usize, columns: usize) -> Self {
        self.lines = lines;
        self.columns = columns;
        self
    }

    /// Set the default message shown when idle.
    ///
    /// # Arguments
    ///
    /// * `message` - Default message text
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn with_default_message(mut self, message: String) -> Self {
        self.default_message = message;
        self
    }

    /// Build the virtual display with configured parameters.
    ///
    /// # Returns
    ///
    /// Returns a new `VirtualDisplay` with the specified configuration.
    pub fn build(self) -> VirtualDisplay {
        VirtualDisplay::new(self.lines, self.columns, self.default_message)
    }
}

impl Default for VirtualDisplayBuilder {
    fn default() -> Self {
        Self {
            lines: DEFAULT_LINES,
            columns: DEFAULT_COLUMNS,
            default_message: "DIGITE SEU CODIGO".to_string(),
        }
    }
}

/// Truncate ASCII text to a maximum number of characters.
///
/// # Arguments
///
/// * `text` - ASCII text to truncate
/// * `max_chars` - Maximum number of characters
///
/// # Returns
///
/// Returns truncated string.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::truncate_text;
///
/// assert_eq!(truncate_text("Sao Paulo", 5), "Sao P");
/// assert_eq!(truncate_text("Liberacao", 7), "Liberac");
/// assert_eq!(truncate_text("Short", 10), "Short");
/// ```
pub fn truncate_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

/// Align ASCII text within a fixed width, padding with spaces.
///
/// # Arguments
///
/// * `text` - ASCII text to align
/// * `width` - Target width in characters
/// * `alignment` - Alignment mode (Left, Center, or Right)
///
/// # Returns
///
/// Returns aligned and padded string exactly `width` characters long.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::{align_text, Alignment};
///
/// assert_eq!(align_text("HELLO", 10, Alignment::Left), "HELLO     ");
/// assert_eq!(align_text("HELLO", 10, Alignment::Center), "  HELLO   ");
/// assert_eq!(align_text("HELLO", 10, Alignment::Right), "     HELLO");
/// ```
pub fn align_text(text: &str, width: usize, alignment: Alignment) -> String {
    let char_count = text.chars().count();

    // If text is longer than width, truncate it
    if char_count >= width {
        return truncate_text(text, width);
    }

    let padding = width - char_count;

    match alignment {
        Alignment::Left => format!("{}{}", text, " ".repeat(padding)),
        Alignment::Right => format!("{}{}", " ".repeat(padding), text),
        Alignment::Center => {
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
    }
}

/// Sanitize text by removing control characters and trimming.
///
/// # Arguments
///
/// * `text` - Text to sanitize
///
/// # Returns
///
/// Returns sanitized string.
fn sanitize_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_display_with_default_message() {
        let display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());

        assert_eq!(display.lines, 2);
        assert_eq!(display.columns, 40);
        assert_eq!(display.get_line(0).unwrap().trim(), "DIGITE SEU CODIGO");
    }

    #[test]
    fn test_set_line_basic() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.set_line(0, "AGUARDE...").unwrap();

        assert_eq!(display.get_line(0).unwrap().trim_end(), "AGUARDE...");
    }

    #[test]
    fn test_set_line_invalid_index() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        let result = display.set_line(5, "TEXT");

        assert!(result.is_err());
        if let Err(Error::InvalidLine { line, max }) = result {
            assert_eq!(line, 5);
            assert_eq!(max, 1);
        } else {
            panic!("Expected InvalidLine error");
        }
    }

    #[test]
    fn test_text_truncation_at_40_characters() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        let long_text =
            "This is a very long text that exceeds the 40 character limit of the display";
        display.set_line(0, long_text).unwrap();

        let result = display.get_line(0).unwrap();
        assert_eq!(result.len(), 40);
        assert_eq!(result, "This is a very long text that exceeds th");
    }

    #[test]
    fn test_ascii_character_counting() {
        let text = "Sao Paulo";
        assert_eq!(text.chars().count(), 9);

        let truncated = truncate_text(text, 5);
        assert_eq!(truncated, "Sao P");
    }

    #[test]
    fn test_text_alignment_left() {
        let result = align_text("HELLO", 10, Alignment::Left);
        assert_eq!(result, "HELLO     ");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_text_alignment_center() {
        let result = align_text("HELLO", 10, Alignment::Center);
        assert_eq!(result, "  HELLO   ");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_text_alignment_center_odd_padding() {
        let result = align_text("HELLO", 11, Alignment::Center);
        assert_eq!(result, "   HELLO   ");
        assert_eq!(result.len(), 11);
    }

    #[test]
    fn test_text_alignment_right() {
        let result = align_text("HELLO", 10, Alignment::Right);
        assert_eq!(result, "     HELLO");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_text_padding_exact_width() {
        let result = align_text("HELLO", 5, Alignment::Left);
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_clear_display() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.set_line(0, "TEXT").unwrap();
        display.clear();

        assert_eq!(display.get_line(0).unwrap().trim(), "");
        assert_eq!(display.get_line(1).unwrap().trim(), "");
    }

    #[test]
    fn test_reset_to_default() {
        let mut display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
        display.set_line(0, "TEMPORARY").unwrap();

        assert!(!display.is_default());

        display.reset_to_default();
        assert!(display.is_default());
        assert_eq!(display.get_line(0).unwrap().trim(), "DIGITE SEU CODIGO");
    }

    #[test]
    fn test_temporary_message_basic() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display
            .show_temporary("ACESSO LIBERADO", Duration::from_secs(5))
            .unwrap();

        assert!(!display.is_default());
        assert!(display.temporary_message.is_some());
    }

    #[test]
    fn test_temporary_message_zero_duration() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        let result = display.show_temporary("TEXT", Duration::from_secs(0));

        assert!(result.is_err());
        if let Err(Error::InvalidDuration) = result {
            // Expected error
        } else {
            panic!("Expected InvalidDuration error");
        }
    }

    #[test]
    fn test_temporary_message_expiration() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display
            .show_temporary("TEMPORARY", Duration::from_millis(50))
            .unwrap();

        assert!(!display.is_default());

        thread::sleep(Duration::from_millis(100));

        let changed = display.update();
        assert!(changed);
        assert!(display.is_default());
    }

    #[test]
    fn test_update_no_change() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        let changed = display.update();

        assert!(!changed);
    }

    #[test]
    fn test_state_machine_integration_idle() {
        let mut display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
        display.update_from_state(&TurnstileState::Idle);

        assert_eq!(display.get_line(0).unwrap().trim(), "DIGITE SEU CODIGO");
    }

    #[test]
    fn test_state_machine_integration_reading() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.update_from_state(&TurnstileState::Reading);

        assert_eq!(display.get_line(0).unwrap().trim(), "AGUARDE...");
        assert_eq!(display.get_line(1).unwrap().trim(), "Lendo credencial");
    }

    #[test]
    fn test_state_machine_integration_validating() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.update_from_state(&TurnstileState::Validating);

        assert_eq!(display.get_line(0).unwrap().trim(), "VALIDANDO...");
        assert_eq!(display.get_line(1).unwrap().trim(), "Aguarde resposta");
    }

    #[test]
    fn test_state_machine_integration_granted() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.update_from_state(&TurnstileState::Granted);

        assert_eq!(display.get_line(0).unwrap().trim(), "ACESSO LIBERADO");
    }

    #[test]
    fn test_state_machine_integration_denied() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.update_from_state(&TurnstileState::Denied);

        assert_eq!(display.get_line(0).unwrap().trim(), "ACESSO NEGADO");
    }

    #[test]
    fn test_state_machine_integration_waiting_rotation() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.update_from_state(&TurnstileState::WaitingRotation);

        assert_eq!(display.get_line(0).unwrap().trim(), "PASSE PELA CATRACA");
    }

    #[test]
    fn test_state_machine_integration_rotation_in_progress() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.update_from_state(&TurnstileState::RotationInProgress);

        assert_eq!(display.get_line(0).unwrap().trim(), "GIRANDO...");
    }

    #[test]
    fn test_state_machine_integration_rotation_completed() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.update_from_state(&TurnstileState::RotationCompleted);

        assert_eq!(display.get_line(0).unwrap().trim(), "OBRIGADO");
    }

    #[test]
    fn test_state_machine_integration_rotation_timeout() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.update_from_state(&TurnstileState::RotationTimeout);

        assert_eq!(display.get_line(0).unwrap().trim(), "TEMPO ESGOTADO");
    }

    #[test]
    fn test_get_line_out_of_bounds() {
        let display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        let result = display.get_line(5);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_all_lines() {
        let display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        let lines = display.get_all_lines();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].len(), 40);
        assert_eq!(lines[1].len(), 40);
    }

    #[test]
    fn test_is_default() {
        let mut display = VirtualDisplay::new(2, 40, "DIGITE SEU CODIGO".to_string());
        assert!(display.is_default());

        display.set_line(0, "OTHER").unwrap();
        assert!(!display.is_default());

        display.reset_to_default();
        assert!(display.is_default());
    }

    #[test]
    fn test_empty_text_handling() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.set_line(0, "").unwrap();

        assert_eq!(display.get_line(0).unwrap().trim(), "");
    }

    #[test]
    fn test_whitespace_only_text() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.set_line(0, "     ").unwrap();

        // Sanitize removes leading/trailing whitespace
        assert_eq!(display.get_line(0).unwrap().trim(), "");
    }

    #[test]
    fn test_control_characters_removal() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.set_line(0, "Hello\nWorld\r\n\tTest").unwrap();

        let result = display.get_line(0).unwrap().trim();
        assert!(!result.contains('\n'));
        assert!(!result.contains('\r'));
        assert!(!result.contains('\t'));
        assert_eq!(result, "HelloWorldTest");
    }

    #[test]
    fn test_builder_default() {
        let display = VirtualDisplay::builder().build();

        assert_eq!(display.lines, 2);
        assert_eq!(display.columns, 40);
        assert_eq!(display.get_line(0).unwrap().trim(), "DIGITE SEU CODIGO");
    }

    #[test]
    fn test_builder_with_size() {
        let display = VirtualDisplay::builder()
            .with_size(4, 20)
            .with_default_message("TEST".to_string())
            .build();

        assert_eq!(display.lines, 4);
        assert_eq!(display.columns, 20);
    }

    #[test]
    fn test_builder_with_custom_message() {
        let display = VirtualDisplay::builder()
            .with_default_message("WELCOME".to_string())
            .build();

        assert_eq!(display.get_line(0).unwrap().trim(), "WELCOME");
    }

    #[test]
    fn test_builder_fluent_api() {
        let display = VirtualDisplay::builder()
            .with_size(2, 40)
            .with_default_message("CUSTOM".to_string())
            .build();

        assert_eq!(display.lines, 2);
        assert_eq!(display.columns, 40);
        assert_eq!(display.get_line(0).unwrap().trim(), "CUSTOM");
    }

    #[test]
    fn test_set_lines_both() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        display.set_lines("LINE 1", "LINE 2").unwrap();

        assert_eq!(display.get_line(0).unwrap().trim_end(), "LINE 1");
        assert_eq!(display.get_line(1).unwrap().trim_end(), "LINE 2");
    }

    #[test]
    fn test_very_long_text_truncation() {
        let mut display = VirtualDisplay::new(2, 40, "IDLE".to_string());
        let very_long = "A".repeat(1000);
        display.set_line(0, &very_long).unwrap();

        let result = display.get_line(0).unwrap();
        assert_eq!(result.len(), 40);
        assert_eq!(result, "A".repeat(40));
    }

    #[test]
    fn test_sanitize_text_control_chars() {
        let result = sanitize_text("Hello\nWorld\r\n\tTest");
        assert_eq!(result, "HelloWorldTest");
    }

    #[test]
    fn test_truncate_text_exact() {
        assert_eq!(truncate_text("Hello", 5), "Hello");
        assert_eq!(truncate_text("Hello", 3), "Hel");
        assert_eq!(truncate_text("Hello", 10), "Hello");
    }
}
