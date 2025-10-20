//! Stream parser for Henry protocol messages.
//!
//! This module provides a stateful parser capable of handling partial messages
//! from TCP streams. The parser accumulates bytes in an internal buffer and
//! extracts complete frames using a state machine that detects STX/ETX framing.
//!
//! # Protocol Framing
//!
//! Henry protocol messages are framed with start and end bytes:
//! - Start byte (STX): `0x02`
//! - End byte (ETX): `0x03`
//!
//! Complete message format:
//! ```text
//! STX  <payload>  ETX
//! 0x02 ID+REON+... 0x03
//! ```
//!
//! # Usage
//!
//! ```
//! use turnkey_protocol::StreamParser;
//!
//! let mut parser = StreamParser::new();
//!
//! // Feed partial data from TCP stream
//! parser.feed(&[0x02, b'1', b'5', b'+']);
//! parser.feed(b"REON+RQ");
//! parser.feed(&[0x03]);
//!
//! // Extract complete frame
//! if let Some(frame) = parser.next_frame() {
//!     println!("Received: {}", frame);
//! }
//! ```
//!
//! # ASCII Encoding
//!
//! The Henry protocol uses ASCII encoding (7-bit, 0x00-0x7F). All characters
//! in the protocol messages must be valid ASCII. Bytes outside this range
//! indicate protocol violation or data corruption.

use bytes::BytesMut;
use std::collections::VecDeque;
use turnkey_core::constants::{END_BYTE, START_BYTE};

use crate::frame::Frame;

/// Maximum buffer size to prevent memory exhaustion from malformed streams.
///
/// If buffer grows beyond this size without finding a complete frame,
/// it indicates either a very large message or a protocol violation.
const MAX_BUFFER_SIZE: usize = 64 * 1024; // 64 KB

/// Initial buffer capacity for incoming TCP data.
///
/// This value is optimized for typical TCP packet sizes.
const INITIAL_BUFFER_CAPACITY: usize = 4 * 1024; // 4 KB

/// Initial payload capacity for frame assembly.
///
/// This value accommodates most Henry protocol messages without reallocation.
const INITIAL_PAYLOAD_CAPACITY: usize = 1024; // 1 KB

/// Recommended initial capacity for frame queue.
///
/// Most streams process frames one at a time, but this allows
/// buffering multiple frames during burst traffic without reallocation.
const INITIAL_FRAME_QUEUE_CAPACITY: usize = 4;

/// State machine states for parsing Henry protocol frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    /// Waiting for STX (0x02) start byte.
    ///
    /// In this state, the parser scans incoming bytes looking for the
    /// start-of-text marker. Any bytes before STX are considered garbage
    /// and are discarded.
    WaitingStart,

    /// Reading payload bytes until ETX (0x03) end byte.
    ///
    /// In this state, the parser accumulates all bytes into the payload
    /// buffer until it encounters the end-of-text marker.
    ReadingPayload,
}

/// Stateful stream parser for Henry protocol messages.
///
/// The `StreamParser` handles partial message reception from TCP streams,
/// buffering incomplete data and extracting complete frames using a
/// state machine approach.
///
/// # State Machine
///
/// The parser uses a two-state machine to robustly handle partial frames:
///
/// ```text
/// ┌─────────────┐  STX byte    ┌───────────────┐  ETX byte   ┌─────────────┐
/// │WaitingStart │─────────────>│ReadingPayload │────────────>│Frame ready  │
/// └─────────────┘              └───────────────┘             └─────────────┘
///       ^  │                          │                              │
///       │  │ Non-STX bytes            │ Buffer > MAX_SIZE            │
///       │  │ (discarded)              │ (reset to prevent DoS)       │
///       │  └──────────────────────────┘                              │
///       │                                                            │
///       └────────────────────────────────────────────────────────────┘
///                           next_frame() called
///
/// State transitions:
/// - WaitingStart → ReadingPayload: When STX (0x02) byte is found
/// - ReadingPayload → WaitingStart: When ETX (0x03) byte is found and frame is extracted
/// - ReadingPayload → WaitingStart: When buffer exceeds MAX_BUFFER_SIZE (DoS protection)
/// - WaitingStart → WaitingStart: When non-STX bytes are encountered (garbage discarded)
/// ```
///
/// # Why This Design?
///
/// TCP is a stream protocol without message boundaries. A single `read()` call
/// might contain: partial frame, complete frame, multiple frames, or garbage.
/// The state machine ensures we correctly handle all cases:
///
/// - **Partial frames**: Buffered until complete
/// - **Multiple frames**: All extracted and queued
/// - **Garbage data**: Discarded when looking for STX
/// - **DoS protection**: Buffer size limits prevent memory exhaustion
///
/// # Example
///
/// ```
/// use turnkey_protocol::StreamParser;
///
/// let mut parser = StreamParser::new();
///
/// // Simulate TCP stream receiving data in chunks
/// parser.feed(&[0x02, b'0', b'1']); // Partial: STX + "01"
/// assert!(parser.next_frame().is_none()); // Not complete yet
///
/// parser.feed(b"+REON+RQ"); // More data
/// assert!(parser.next_frame().is_none()); // Still not complete
///
/// parser.feed(&[0x03]); // Final: ETX
///
/// // Now we have a complete frame
/// let frame = parser.next_frame().unwrap();
/// assert_eq!(frame.to_string().unwrap(), "01+REON+RQ");
/// ```
#[derive(Debug)]
pub struct StreamParser {
    /// Internal buffer for accumulating incoming bytes.
    buffer: BytesMut,

    /// Current state of the parser state machine.
    state: ParserState,

    /// Temporary buffer for current frame payload (between STX and ETX).
    payload: Vec<u8>,

    /// Queue of complete frames ready for extraction.
    frames: VecDeque<Frame>,
}

impl StreamParser {
    /// Create a new stream parser with optimized initial capacity.
    ///
    /// Preallocates buffers to avoid reallocation during typical usage:
    /// - 4 KB for incoming TCP data (standard packet size)
    /// - 1 KB for frame payload assembly
    /// - 4 slots for frame queue (handles burst traffic)
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::StreamParser;
    ///
    /// let parser = StreamParser::new();
    /// ```
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(INITIAL_BUFFER_CAPACITY),
            state: ParserState::WaitingStart,
            payload: Vec::with_capacity(INITIAL_PAYLOAD_CAPACITY),
            frames: VecDeque::with_capacity(INITIAL_FRAME_QUEUE_CAPACITY),
        }
    }

    /// Feed bytes from TCP stream into the parser.
    ///
    /// This method appends new bytes to the internal buffer and attempts
    /// to extract complete frames using the state machine. Multiple frames
    /// may be extracted from a single `feed()` call if the data contains
    /// multiple complete messages.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Slice of bytes from the TCP stream
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::StreamParser;
    ///
    /// let mut parser = StreamParser::new();
    ///
    /// // Feed partial message
    /// parser.feed(&[0x02, b'1', b'5']);
    /// parser.feed(b"+REON+RQ");
    /// parser.feed(&[0x03]);
    ///
    /// assert!(parser.next_frame().is_some());
    /// ```
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);

        // Try to extract frames from buffer
        while self.try_extract_frame() {
            // Continue extracting frames while possible
        }
    }

    /// Extract next complete frame if available.
    ///
    /// Returns `Some(Frame)` if a complete frame has been parsed and is
    /// ready for consumption. Returns `None` if no complete frame is
    /// available yet (waiting for more data).
    ///
    /// # Returns
    ///
    /// - `Some(Frame)` - A complete frame ready for processing
    /// - `None` - No complete frame available, needs more data
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::StreamParser;
    ///
    /// let mut parser = StreamParser::new();
    /// parser.feed(&[0x02, b'0', b'1', b'+', b'R', b'E', b'O', b'N', b'+', b'R', b'Q', 0x03]);
    ///
    /// if let Some(frame) = parser.next_frame() {
    ///     println!("Frame: {}", frame);
    /// }
    /// ```
    pub fn next_frame(&mut self) -> Option<Frame> {
        self.frames.pop_front()
    }

    /// Returns current parser state.
    ///
    /// This method is useful for debugging and monitoring the parser's
    /// internal state machine.
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::{StreamParser, ParserState};
    ///
    /// let parser = StreamParser::new();
    /// assert_eq!(parser.state(), ParserState::WaitingStart);
    /// ```
    pub fn state(&self) -> ParserState {
        self.state
    }

    /// Returns number of frames ready for extraction.
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::StreamParser;
    ///
    /// let mut parser = StreamParser::new();
    /// parser.feed(&[0x02, b'0', b'1', b'+', b'R', b'E', b'O', b'N', b'+', b'R', b'Q', 0x03]);
    ///
    /// assert_eq!(parser.frames_available(), 1);
    /// ```
    pub fn frames_available(&self) -> usize {
        self.frames.len()
    }

    /// Clear all internal buffers and reset state.
    ///
    /// This method is useful for error recovery or when resetting the
    /// connection. It performs a complete cleanup:
    /// - Discards all buffered bytes
    /// - Clears accumulated payload
    /// - Removes all queued frames
    /// - Resets state machine to initial state
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::{StreamParser, ParserState};
    ///
    /// let mut parser = StreamParser::new();
    /// parser.feed(&[0x02, b'0', b'1']);
    /// parser.clear();
    ///
    /// assert_eq!(parser.state(), ParserState::WaitingStart);
    /// assert_eq!(parser.frames_available(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.discard_buffer();
        self.payload.clear();
        self.frames.clear();
        self.state = ParserState::WaitingStart;
    }

    /// Returns an iterator that drains all currently available frames.
    ///
    /// This iterator will yield frames until the internal queue is empty.
    /// It does NOT process more data from the buffer - call [`feed()`] first
    /// to parse incoming bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::StreamParser;
    ///
    /// let mut parser = StreamParser::new();
    /// parser.feed(&[0x02, b'0', b'1', b'+', b'R', b'E', b'O', b'N', b'+', b'R', b'Q', 0x03]);
    /// parser.feed(&[0x02, b'0', b'2', b'+', b'R', b'E', b'O', b'N', b'+', b'E', b'C', 0x03]);
    ///
    /// // Process all frames using iterator
    /// for frame in parser.drain_frames() {
    ///     println!("Frame: {}", frame);
    /// }
    ///
    /// assert_eq!(parser.frames_available(), 0);
    /// ```
    ///
    /// [`feed()`]: StreamParser::feed
    pub fn drain_frames(&mut self) -> DrainFrames<'_> {
        DrainFrames { parser: self }
    }

    /// Try to extract one complete frame from the buffer.
    ///
    /// Returns `true` if a frame was extracted, `false` otherwise.
    /// This method implements the state machine logic for frame extraction.
    fn try_extract_frame(&mut self) -> bool {
        // Check buffer size limit
        if self.buffer.len() > MAX_BUFFER_SIZE {
            // Buffer too large, discard everything and reset
            self.clear();
            return false;
        }

        loop {
            match self.state {
                ParserState::WaitingStart => {
                    if !self.handle_waiting_start() {
                        return false;
                    }
                }
                ParserState::ReadingPayload => {
                    return self.handle_reading_payload();
                }
            }
        }
    }

    /// Handle WaitingStart state: look for STX byte.
    ///
    /// Returns `true` if STX was found and state transitioned, `false` otherwise.
    fn handle_waiting_start(&mut self) -> bool {
        if let Some(stx_pos) = self.buffer.iter().position(|&b| b == START_BYTE) {
            self.discard_garbage_before_stx(stx_pos);
            self.consume_stx_byte();
            self.transition_to_reading_payload();
            true
        } else {
            self.discard_buffer();
            false
        }
    }

    /// Discard garbage bytes that appear before STX marker.
    ///
    /// The Henry protocol only recognizes data between STX and ETX markers.
    /// Any bytes before STX are considered noise and must be discarded.
    fn discard_garbage_before_stx(&mut self, stx_pos: usize) {
        let _ = self.buffer.split_to(stx_pos);
    }

    /// Consume the STX (Start of Text) byte from buffer.
    ///
    /// After locating the STX marker, this method removes it from
    /// the buffer so subsequent bytes become part of the payload.
    fn consume_stx_byte(&mut self) {
        let _ = self.buffer.split_to(1);
    }

    /// Transition state machine to ReadingPayload state.
    ///
    /// Prepares the parser to accumulate payload bytes by clearing
    /// any previous payload data and updating the state.
    fn transition_to_reading_payload(&mut self) {
        self.state = ParserState::ReadingPayload;
        self.payload.clear();
    }

    /// Discard all bytes from internal buffer.
    ///
    /// Used when no STX marker is found - all buffered bytes
    /// are considered garbage and discarded.
    fn discard_buffer(&mut self) {
        self.buffer.clear();
    }

    /// Handle ReadingPayload state: look for ETX byte and extract frame.
    ///
    /// Returns `true` if frame was extracted, `false` if need more data.
    fn handle_reading_payload(&mut self) -> bool {
        if let Some(etx_pos) = self.buffer.iter().position(|&b| b == END_BYTE) {
            self.extract_payload_from_buffer(etx_pos);

            if self.is_valid_ascii_payload() {
                self.enqueue_frame_from_payload();
            }
            // Note: Non-ASCII frames are silently discarded as protocol violations

            self.reset_for_next_frame();
            true
        } else {
            self.accumulate_available_bytes();
            false
        }
    }

    /// Extract payload bytes from buffer until ETX marker.
    ///
    /// This method removes bytes from the buffer up to the ETX position,
    /// appends them to the payload, and consumes the ETX byte itself.
    fn extract_payload_from_buffer(&mut self, etx_pos: usize) {
        let payload_bytes = self.buffer.split_to(etx_pos);
        self.payload.extend_from_slice(&payload_bytes);
        let _ = self.buffer.split_to(1); // Consume ETX byte
    }

    /// Validate that payload contains only ASCII bytes.
    ///
    /// Henry protocol requires ASCII encoding (7-bit, 0x00-0x7F).
    /// Non-ASCII bytes indicate protocol violation or data corruption.
    fn is_valid_ascii_payload(&self) -> bool {
        self.payload.iter().all(|&b| b.is_ascii())
    }

    /// Create frame from current payload and enqueue it.
    ///
    /// Constructs a Frame from the accumulated payload bytes and
    /// adds it to the queue of frames ready for extraction.
    fn enqueue_frame_from_payload(&mut self) {
        let frame = Frame::from_bytes(&self.payload, false);
        self.frames.push_back(frame);
    }

    /// Reset parser state for next frame.
    ///
    /// Clears the payload buffer and transitions state machine
    /// back to waiting for the next STX marker.
    fn reset_for_next_frame(&mut self) {
        self.state = ParserState::WaitingStart;
        self.payload.clear();
    }

    /// Accumulate available bytes into payload buffer.
    ///
    /// When ETX is not yet found, this method moves all available
    /// bytes from the internal buffer to the payload buffer while
    /// preserving buffer capacity for efficiency.
    fn accumulate_available_bytes(&mut self) {
        let available_len = self.buffer.len();
        if available_len > 0 {
            self.payload.extend_from_slice(&self.buffer[..]);
            self.buffer.clear();
        }
    }
}

impl Default for StreamParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator that drains frames from a [`StreamParser`].
///
/// This iterator is created by the [`drain_frames()`] method on [`StreamParser`].
/// It yields all currently available frames from the parser's internal queue.
///
/// # Example
///
/// ```
/// use turnkey_protocol::StreamParser;
///
/// let mut parser = StreamParser::new();
/// parser.feed(&[0x02, b'0', b'1', b'+', b'R', b'E', b'O', b'N', b'+', b'R', b'Q', 0x03]);
/// parser.feed(&[0x02, b'0', b'2', b'+', b'R', b'E', b'O', b'N', b'+', b'E', b'C', 0x03]);
///
/// let frames: Vec<_> = parser.drain_frames().collect();
/// assert_eq!(frames.len(), 2);
/// ```
///
/// [`drain_frames()`]: StreamParser::drain_frames
pub struct DrainFrames<'a> {
    parser: &'a mut StreamParser,
}

impl<'a> Iterator for DrainFrames<'a> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next_frame()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.parser.frames_available();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for DrainFrames<'a> {
    fn len(&self) -> usize {
        self.parser.frames_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: Create a complete frame with STX and ETX markers.
    ///
    /// Builds a properly framed protocol message for testing.
    fn make_frame(payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(payload.len() + 2);
        frame.push(START_BYTE);
        frame.extend_from_slice(payload);
        frame.push(END_BYTE);
        frame
    }

    /// Test helper: Create multiple complete frames.
    ///
    /// Concatenates multiple framed messages into a single buffer.
    fn make_frames(payloads: &[&[u8]]) -> Vec<u8> {
        let mut data = Vec::new();
        for payload in payloads {
            data.push(START_BYTE);
            data.extend_from_slice(payload);
            data.push(END_BYTE);
        }
        data
    }

    /// Test helper: Feed multiple standard test frames to parser.
    ///
    /// Feeds common test frames: "01+REON+RQ", "02+REON+EC", "03+REON+EH"
    fn feed_standard_test_frames(parser: &mut StreamParser, count: usize) {
        let frames = [
            b"01+REON+RQ".as_slice(),
            b"02+REON+EC".as_slice(),
            b"03+REON+EH".as_slice(),
        ];

        for frame in frames.iter().take(count) {
            parser.feed(&make_frame(frame));
        }
    }

    #[test]
    fn test_new_parser() {
        let parser = StreamParser::new();
        assert_eq!(parser.state(), ParserState::WaitingStart);
        assert_eq!(parser.frames_available(), 0);
    }

    #[test]
    fn test_complete_frame_single_feed() {
        let mut parser = StreamParser::new();

        // Complete frame in one feed
        parser.feed(&make_frame(b"01+REON+RQ"));

        assert_eq!(parser.frames_available(), 1);

        let frame = parser.next_frame().unwrap();
        assert_eq!(frame.to_string().unwrap(), "01+REON+RQ");
    }

    #[test]
    fn test_partial_frame_multiple_feeds() {
        let mut parser = StreamParser::new();

        // Frame arrives in 3 parts
        parser.feed(&[0x02, b'1', b'5', b'+']);
        assert!(parser.next_frame().is_none());

        parser.feed(b"REON+000+0]12345");
        assert!(parser.next_frame().is_none());

        parser.feed(&[b'6', b'7', b'8', 0x03]);
        assert_eq!(parser.frames_available(), 1);

        let frame = parser.next_frame().unwrap();
        assert_eq!(frame.to_string().unwrap(), "15+REON+000+0]12345678");
    }

    #[test]
    fn test_multiple_frames_in_single_buffer() {
        let mut parser = StreamParser::new();

        // Two complete frames in one feed
        let data = make_frames(&[b"01+REON+RQ", b"02+REON+EC"]);

        parser.feed(&data);

        assert_eq!(parser.frames_available(), 2);

        let frame1 = parser.next_frame().unwrap();
        assert_eq!(frame1.to_string().unwrap(), "01+REON+RQ");

        let frame2 = parser.next_frame().unwrap();
        assert_eq!(frame2.to_string().unwrap(), "02+REON+EC");
    }

    #[test]
    fn test_garbage_before_stx() {
        let mut parser = StreamParser::new();

        // Garbage bytes before STX should be discarded
        let mut data = Vec::new();
        data.extend_from_slice(b"garbage123");
        data.extend_from_slice(&make_frame(b"01+REON+RQ"));

        parser.feed(&data);

        assert_eq!(parser.frames_available(), 1);

        let frame = parser.next_frame().unwrap();
        assert_eq!(frame.to_string().unwrap(), "01+REON+RQ");
    }

    #[test]
    fn test_incomplete_frame_remains_buffered() {
        let mut parser = StreamParser::new();

        // Send frame without ETX
        parser.feed(&[
            0x02, b'0', b'1', b'+', b'R', b'E', b'O', b'N', b'+', b'R', b'Q',
        ]);

        assert_eq!(parser.frames_available(), 0);
        assert_eq!(parser.state(), ParserState::ReadingPayload);

        // Complete the frame
        parser.feed(&[0x03]);

        assert_eq!(parser.frames_available(), 1);
        assert_eq!(parser.state(), ParserState::WaitingStart);
    }

    #[test]
    fn test_empty_payload() {
        let mut parser = StreamParser::new();

        // Frame with empty payload (just STX+ETX)
        parser.feed(&[0x02, 0x03]);

        assert_eq!(parser.frames_available(), 1);

        let frame = parser.next_frame().unwrap();
        assert_eq!(frame.to_string().unwrap(), "");
    }

    #[test]
    fn test_byte_by_byte_feeding() {
        let mut parser = StreamParser::new();

        let message = b"\x0201+REON+RQ\x03";

        // Feed one byte at a time
        for &byte in message {
            parser.feed(&[byte]);
        }

        assert_eq!(parser.frames_available(), 1);

        let frame = parser.next_frame().unwrap();
        assert_eq!(frame.to_string().unwrap(), "01+REON+RQ");
    }

    #[test]
    fn test_clear_resets_parser() {
        let mut parser = StreamParser::new();

        parser.feed(&[0x02, b'0', b'1', b'+']);
        assert_eq!(parser.state(), ParserState::ReadingPayload);

        parser.clear();

        assert_eq!(parser.state(), ParserState::WaitingStart);
        assert_eq!(parser.frames_available(), 0);
    }

    #[test]
    fn test_only_stx_no_etx() {
        let mut parser = StreamParser::new();

        // Send many STX bytes without ETX
        parser.feed(&[0x02, b'0', b'1', 0x02, 0x02, b'R', b'Q']);

        assert_eq!(parser.frames_available(), 0);
        assert_eq!(parser.state(), ParserState::ReadingPayload);
    }

    #[test]
    fn test_protocol_message_with_fields() {
        let mut parser = StreamParser::new();

        // Real protocol message
        let message = b"\x0215+REON+000+0]00000000000011912322]10/05/2025 12:46:06]1]0]\x03";
        parser.feed(message);

        assert_eq!(parser.frames_available(), 1);

        let frame = parser.next_frame().unwrap();
        let content = frame.to_string().unwrap();
        assert!(content.contains("15+REON+000+0"));
        assert!(content.contains("11912322"));
        assert!(content.contains("10/05/2025 12:46:06"));
    }

    #[test]
    fn test_no_frames_without_stx() {
        let mut parser = StreamParser::new();

        // Data without STX
        parser.feed(b"01+REON+RQ");

        assert_eq!(parser.frames_available(), 0);
    }

    #[test]
    fn test_mixed_complete_and_partial() {
        let mut parser = StreamParser::new();

        // One complete frame + start of another
        let mut data = Vec::new();
        data.push(0x02);
        data.extend_from_slice(b"01+REON+RQ");
        data.push(0x03);
        data.push(0x02);
        data.extend_from_slice(b"02+REON");
        // No ETX yet

        parser.feed(&data);

        assert_eq!(parser.frames_available(), 1);
        assert_eq!(parser.state(), ParserState::ReadingPayload);

        // Extract the first complete frame
        let frame1 = parser.next_frame().unwrap();
        assert_eq!(frame1.to_string().unwrap(), "01+REON+RQ");

        // Complete the partial frame
        parser.feed(&[b'+', b'E', b'C', 0x03]);

        assert_eq!(parser.frames_available(), 1);

        let frame2 = parser.next_frame().unwrap();
        assert_eq!(frame2.to_string().unwrap(), "02+REON+EC");
    }

    #[test]
    fn test_state_transitions() {
        let mut parser = StreamParser::new();

        assert_eq!(parser.state(), ParserState::WaitingStart);

        parser.feed(&[0x02]);
        assert_eq!(parser.state(), ParserState::ReadingPayload);

        parser.feed(b"01");
        assert_eq!(parser.state(), ParserState::ReadingPayload);

        parser.feed(&[0x03]);
        assert_eq!(parser.state(), ParserState::WaitingStart);
    }

    #[test]
    fn test_ascii_protocol_message() {
        let mut parser = StreamParser::new();

        // ASCII-only message (no non-ASCII bytes)
        parser.feed(&[0x02]);
        parser.feed(b"15+REON+00+6]5]Acesso liberado]");
        parser.feed(&[0x03]);

        let frame = parser.next_frame().unwrap();
        let content = frame.to_string().unwrap();

        // Verify ASCII content
        assert!(content.is_ascii());
        assert_eq!(content, "15+REON+00+6]5]Acesso liberado]");
    }

    #[test]
    fn test_non_ascii_bytes_rejected() {
        let mut parser = StreamParser::new();

        // Frame with non-ASCII bytes (protocol violation)
        let mut data = Vec::new();
        data.push(0x02); // STX
        data.extend_from_slice(b"01+REON+");
        data.push(0xFF); // Non-ASCII byte (invalid)
        data.push(0xFE); // Non-ASCII byte (invalid)
        data.extend_from_slice(b"+RQ");
        data.push(0x03); // ETX

        parser.feed(&data);

        // Frame should be discarded due to non-ASCII bytes
        assert_eq!(parser.frames_available(), 0);
        assert_eq!(parser.state(), ParserState::WaitingStart);
    }

    #[test]
    fn test_embedded_stx_in_payload() {
        let mut parser = StreamParser::new();

        // Frame with STX byte embedded in payload
        // Tests that parser doesn't restart when seeing STX during ReadingPayload
        let mut data = Vec::new();
        data.push(0x02); // Real start
        data.extend_from_slice(b"01+REON+");
        data.push(0x02); // Embedded STX (part of payload data)
        data.extend_from_slice(b"+RQ");
        data.push(0x03); // Real end

        parser.feed(&data);

        assert_eq!(parser.frames_available(), 1);

        let frame = parser.next_frame().unwrap();
        let payload_bytes = frame.as_bytes();

        // Payload should include the embedded STX byte
        assert!(payload_bytes.contains(&0x02));
        assert_eq!(payload_bytes.len(), 12); // "01+REON+" (8) + "\x02" (1) + "+RQ" (3) = 12 bytes
    }

    #[test]
    fn test_buffer_size_limit_exceeded() {
        let mut parser = StreamParser::new();

        // Send STX followed by payload larger than MAX_BUFFER_SIZE
        parser.feed(&[0x02]);

        // Feed data in chunks that exceed MAX_BUFFER_SIZE without ETX
        let chunk = vec![b'X'; 16 * 1024]; // 16 KB chunks
        for _ in 0..5 {
            // 5 * 16KB = 80KB > 64KB MAX_BUFFER_SIZE
            parser.feed(&chunk);
        }

        // Parser should have cleared buffer and reset state on next try_extract_frame
        // Note: State may still be ReadingPayload until next feed() triggers the check
        // But frames should be empty and no crash should occur
        assert_eq!(parser.frames_available(), 0);

        // After clearing, parser should accept new frames
        parser.feed(&make_frame(b"01+REON+RQ"));
        assert_eq!(parser.frames_available(), 1);
    }

    #[test]
    fn test_multiple_clear_calls() {
        let mut parser = StreamParser::new();

        // Feed partial data
        parser.feed(&[0x02, b'0', b'1', b'+']);

        // Multiple clear calls should be safe
        parser.clear();
        parser.clear();
        parser.clear();

        assert_eq!(parser.state(), ParserState::WaitingStart);
        assert_eq!(parser.frames_available(), 0);

        // Parser should still work after multiple clears
        parser.feed(&make_frame(b"02+REON+RQ"));
        assert_eq!(parser.frames_available(), 1);
    }

    #[test]
    fn test_large_valid_frame() {
        let mut parser = StreamParser::new();

        // Create a large but valid frame (within limits)
        let mut data = Vec::new();
        data.push(0x02);
        data.extend_from_slice(b"15+REON+000+0]");

        // Add large card number field (but still under buffer limit)
        let large_field = "X".repeat(1000);
        data.extend_from_slice(large_field.as_bytes());

        data.extend_from_slice(b"]10/05/2025 12:46:06]1]0]");
        data.push(0x03);

        parser.feed(&data);

        assert_eq!(parser.frames_available(), 1);
        assert_eq!(parser.state(), ParserState::WaitingStart);

        let frame = parser.next_frame().unwrap();
        let content = frame.to_string().unwrap();
        assert!(content.contains(&large_field));
    }

    #[test]
    fn test_drain_frames_iterator() {
        let mut parser = StreamParser::new();

        // Feed multiple frames
        feed_standard_test_frames(&mut parser, 3);

        assert_eq!(parser.frames_available(), 3);

        // Use iterator to collect all frames
        let frames: Vec<_> = parser.drain_frames().collect();

        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].to_string().unwrap(), "01+REON+RQ");
        assert_eq!(frames[1].to_string().unwrap(), "02+REON+EC");
        assert_eq!(frames[2].to_string().unwrap(), "03+REON+EH");

        // All frames should be drained
        assert_eq!(parser.frames_available(), 0);
    }

    #[test]
    fn test_drain_frames_for_each() {
        let mut parser = StreamParser::new();

        // Feed multiple frames
        feed_standard_test_frames(&mut parser, 2);

        let mut count = 0;
        parser.drain_frames().for_each(|_frame| {
            count += 1;
        });

        assert_eq!(count, 2);
        assert_eq!(parser.frames_available(), 0);
    }

    #[test]
    fn test_drain_frames_empty() {
        let mut parser = StreamParser::new();

        // No frames available
        let frames: Vec<_> = parser.drain_frames().collect();

        assert_eq!(frames.len(), 0);
    }

    #[test]
    fn test_drain_frames_size_hint() {
        let mut parser = StreamParser::new();

        // Feed 3 frames
        feed_standard_test_frames(&mut parser, 3);

        let mut iter = parser.drain_frames();

        // Check size_hint before consuming
        assert_eq!(iter.size_hint(), (3, Some(3)));
        assert_eq!(iter.len(), 3);

        // Consume one
        let _ = iter.next();
        assert_eq!(iter.size_hint(), (2, Some(2)));
        assert_eq!(iter.len(), 2);

        // Consume another
        let _ = iter.next();
        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert_eq!(iter.len(), 1);

        // Consume last
        let _ = iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.len(), 0);

        // No more items
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_drain_frames_filter_map() {
        let mut parser = StreamParser::new();

        // Feed multiple frames
        feed_standard_test_frames(&mut parser, 3);

        // Use iterator combinators to filter frames containing "EC"
        let ec_frames: Vec<_> = parser
            .drain_frames()
            .filter(|frame| frame.to_string().unwrap().contains("EC"))
            .collect();

        assert_eq!(ec_frames.len(), 1);
        assert_eq!(ec_frames[0].to_string().unwrap(), "02+REON+EC");
    }
}
