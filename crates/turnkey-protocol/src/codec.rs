//! Tokio codec for Henry protocol message framing.
//!
//! This module provides a Tokio-compatible codec that integrates the Henry protocol
//! with async TCP communication, enabling automatic message encoding and decoding
//! using Tokio's `Framed` streams.
//!
//! # Overview
//!
//! The `HenryCodec` wraps the [`StreamParser`] to provide a thin integration layer
//! with Tokio's codec traits. It implements:
//! - [`Decoder`]: Extracts complete messages from TCP byte streams
//! - [`Encoder<Message>`]: Converts messages to wire format with framing
//!
//! # Architecture
//!
//! ```text
//! TCP Stream -> Decoder -> Message (parsed)
//! Message -> Encoder -> TCP Stream (with STX/ETX framing)
//! ```
//!
//! The codec leverages existing infrastructure:
//! - [`StreamParser`]: Handles buffering and state machine for partial messages
//! - [`Frame`]: Manages wire format conversion and framing
//! - [`Message`]: Provides high-level protocol representation
//!
//! # Usage with Tokio Framed
//!
//! ```rust,no_run
//! use tokio::net::TcpStream;
//! use tokio_util::codec::Framed;
//! use turnkey_protocol::{HenryCodec, Message, CommandCode, FieldData, MessageBuilder};
//! use turnkey_core::DeviceId;
//! use futures::{SinkExt, StreamExt};
//!
//! # async fn example() -> turnkey_core::Result<()> {
//! // Connect to device
//! let stream = TcpStream::connect("127.0.0.1:3000").await?;
//! let mut framed = Framed::new(stream, HenryCodec::new());
//!
//! // Send message
//! let device_id = DeviceId::new(15)?;
//! let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
//!     .build()?;
//! framed.send(msg).await?;
//!
//! // Receive response
//! if let Some(Ok(response)) = framed.next().await {
//!     println!("Received: {:?}", response);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # DoS Protection
//!
//! The codec includes protection against denial-of-service attacks:
//! - Maximum frame size limit (default: 64 KB)
//! - Buffer size limits in [`StreamParser`]
//! - Automatic buffer cleanup after frame extraction
//!
//! # Performance
//!
//! The codec is optimized for high throughput:
//! - Zero-copy buffer management using [`BytesMut`]
//! - Preallocated buffers to minimize allocations
//! - Target: 1000+ messages/second
//! - Single-pass parsing with state machine
//!
//! # Error Handling
//!
//! Decode errors can occur when:
//! - Frame exceeds maximum size
//! - Invalid UTF-8 in message
//! - Protocol format violations
//! - Invalid device ID or command code
//!
//! The codec returns errors without panicking, allowing the application
//! to handle protocol violations gracefully.

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use crate::{Frame, Message, StreamParser};
use turnkey_core::{Error, Result};

/// Default maximum frame size in bytes (64 KB).
///
/// This limit prevents denial-of-service attacks by rejecting frames
/// that would consume excessive memory. The limit is generous enough
/// for all legitimate Henry protocol messages while protecting against
/// malicious inputs.
const DEFAULT_MAX_FRAME_SIZE: usize = 64 * 1024;

/// Tokio codec for Henry protocol messages.
///
/// `HenryCodec` integrates the Henry protocol with Tokio's async I/O
/// by implementing the [`Decoder`] and [`Encoder`] traits. It wraps
/// the [`StreamParser`] to handle message framing and buffering.
///
/// # Example
///
/// ```rust,no_run
/// use tokio::net::TcpStream;
/// use tokio_util::codec::Framed;
/// use turnkey_protocol::HenryCodec;
/// use futures::StreamExt;
///
/// # async fn example() -> turnkey_core::Result<()> {
/// let stream = TcpStream::connect("127.0.0.1:3000").await?;
/// let mut framed = Framed::new(stream, HenryCodec::new());
///
/// // Read messages from stream
/// while let Some(result) = framed.next().await {
///     match result {
///         Ok(message) => println!("Received: {:?}", message),
///         Err(e) => eprintln!("Error: {}", e),
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct HenryCodec {
    /// Stream parser for handling partial messages and framing.
    parser: StreamParser,

    /// Maximum allowed frame size in bytes.
    ///
    /// Frames exceeding this size will be rejected with an error
    /// to prevent denial-of-service attacks.
    max_frame_size: usize,
}

impl HenryCodec {
    /// Create a new codec with default maximum frame size.
    ///
    /// The default maximum frame size is 64 KB, which is sufficient
    /// for all legitimate Henry protocol messages.
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::HenryCodec;
    ///
    /// let codec = HenryCodec::new();
    /// ```
    pub fn new() -> Self {
        Self {
            parser: StreamParser::new(),
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
        }
    }

    /// Create a new codec with custom maximum frame size.
    ///
    /// Use this constructor if you need to adjust the frame size limit
    /// for your specific use case.
    ///
    /// # Arguments
    ///
    /// * `max_frame_size` - Maximum allowed frame size in bytes
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::HenryCodec;
    ///
    /// // Allow larger frames (128 KB)
    /// let codec = HenryCodec::with_max_frame_size(128 * 1024);
    /// ```
    pub fn with_max_frame_size(max_frame_size: usize) -> Self {
        Self {
            parser: StreamParser::new(),
            max_frame_size,
        }
    }

    /// Get the current maximum frame size.
    pub fn max_frame_size(&self) -> usize {
        self.max_frame_size
    }
}

impl Default for HenryCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for HenryCodec {
    type Item = Message;
    type Error = Error;

    /// Decode a message from the byte stream.
    ///
    /// This method feeds bytes from the source buffer to the internal
    /// [`StreamParser`], extracts complete frames, and converts them
    /// to [`Message`] objects.
    ///
    /// # Arguments
    ///
    /// * `src` - Source buffer containing bytes from the TCP stream
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Message))` - A complete message was decoded
    /// - `Ok(None)` - Need more data to complete the message
    /// - `Err(Error)` - Protocol violation or invalid message
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The frame exceeds `max_frame_size`
    /// - The frame contains invalid UTF-8
    /// - The message format is invalid
    /// - The device ID or command code is invalid
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use tokio_util::codec::Decoder;
    /// use turnkey_protocol::HenryCodec;
    ///
    /// let mut codec = HenryCodec::new();
    /// let mut buffer = BytesMut::from(&b"\x0215+REON+RQ\x03"[..]);
    ///
    /// match codec.decode(&mut buffer) {
    ///     Ok(Some(msg)) => println!("Decoded: {:?}", msg),
    ///     Ok(None) => println!("Need more data"),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        // Feed new bytes to the parser
        if !src.is_empty() {
            // StreamParser.feed() copies bytes to internal buffer for state machine processing.
            // We clear src because all bytes are now owned by the parser's internal buffer.
            self.parser.feed(src);
            src.clear();
        }

        // Try to extract a complete frame
        if let Some(frame) = self.parser.next_frame() {
            // Check frame size limit.
            // Note: StreamParser enforces MAX_BUFFER_SIZE (64KB) during parsing,
            // providing first-line defense against DoS. This check validates the
            // complete frame against the codec's configured limit.
            if frame.size() > self.max_frame_size {
                return Err(Error::FrameTooLarge {
                    size: frame.size(),
                    max_size: self.max_frame_size,
                });
            }

            // Convert frame to message
            let message = Message::try_from(frame)?;
            Ok(Some(message))
        } else {
            // No complete frame available yet
            Ok(None)
        }
    }
}

impl Encoder<Message> for HenryCodec {
    type Error = Error;

    /// Encode a message to the byte stream.
    ///
    /// This method converts a [`Message`] to a [`Frame`], adds STX/ETX
    /// framing, and writes the result to the destination buffer.
    ///
    /// # Arguments
    ///
    /// * `item` - The message to encode
    /// * `dst` - Destination buffer for the encoded bytes
    ///
    /// # Returns
    ///
    /// - `Ok(())` - Message was successfully encoded
    /// - `Err(Error)` - Encoding failed
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The resulting frame exceeds `max_frame_size`
    /// - Memory allocation fails
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use tokio_util::codec::Encoder;
    /// use turnkey_protocol::{HenryCodec, Message, CommandCode, MessageBuilder};
    /// use turnkey_core::DeviceId;
    ///
    /// # fn example() -> turnkey_core::Result<()> {
    /// let mut codec = HenryCodec::new();
    /// let mut buffer = BytesMut::new();
    ///
    /// let device_id = DeviceId::new(15)?;
    /// let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
    ///     .build()?;
    ///
    /// codec.encode(msg, &mut buffer)?;
    ///
    /// // Buffer now contains: \x0215+REON+RQ\x03
    /// assert_eq!(buffer[0], 0x02); // STX
    /// assert_eq!(buffer[buffer.len() - 1], 0x03); // ETX
    /// # Ok(())
    /// # }
    /// ```
    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<()> {
        // Convert message to frame
        let frame = Frame::from(item);

        // Add STX/ETX framing
        let framed = frame.with_framing();

        // Check size limit before writing
        if framed.size() > self.max_frame_size {
            return Err(Error::FrameTooLarge {
                size: framed.size(),
                max_size: self.max_frame_size,
            });
        }

        // Write framed bytes to destination buffer
        dst.extend_from_slice(framed.as_bytes());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandCode, FieldData, MessageBuilder};
    use turnkey_core::DeviceId;

    #[test]
    fn test_codec_new() {
        let codec = HenryCodec::new();
        assert_eq!(codec.max_frame_size(), DEFAULT_MAX_FRAME_SIZE);
    }

    #[test]
    fn test_codec_with_custom_max_size() {
        let custom_size = 128 * 1024;
        let codec = HenryCodec::with_max_frame_size(custom_size);
        assert_eq!(codec.max_frame_size(), custom_size);
    }

    #[test]
    fn test_codec_default() {
        let codec = HenryCodec::default();
        assert_eq!(codec.max_frame_size(), DEFAULT_MAX_FRAME_SIZE);
    }

    #[test]
    fn test_decode_complete_message() {
        let mut codec = HenryCodec::new();
        let mut buffer = BytesMut::from(&b"\x0215+REON+RQ\x03"[..]);

        let result = codec.decode(&mut buffer);
        assert!(result.is_ok());

        let message = result.unwrap();
        assert!(message.is_some());

        let msg = message.unwrap();
        assert_eq!(msg.device_id.as_u8(), 15);
        assert_eq!(msg.command, CommandCode::QueryStatus);
    }

    #[test]
    fn test_decode_partial_message() {
        let mut codec = HenryCodec::new();
        let mut buffer = BytesMut::from(&b"\x0215+REON"[..]);

        let result = codec.decode(&mut buffer);
        assert!(result.is_ok());

        let message = result.unwrap();
        assert!(message.is_none()); // Not complete yet
    }

    #[test]
    fn test_decode_multiple_messages_in_buffer() {
        let mut codec = HenryCodec::new();
        let mut buffer = BytesMut::from(&b"\x0215+REON+RQ\x03\x0216+REON+RQ\x03"[..]);

        // First message
        let result1 = codec.decode(&mut buffer);
        assert!(result1.is_ok());
        let msg1 = result1.unwrap();
        assert!(msg1.is_some());
        assert_eq!(msg1.unwrap().device_id.as_u8(), 15);

        // Second message
        let result2 = codec.decode(&mut buffer);
        assert!(result2.is_ok());
        let msg2 = result2.unwrap();
        assert!(msg2.is_some());
        assert_eq!(msg2.unwrap().device_id.as_u8(), 16);
    }

    #[test]
    fn test_decode_empty_buffer() {
        let mut codec = HenryCodec::new();
        let mut buffer = BytesMut::new();

        let result = codec.decode(&mut buffer);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_decode_frame_too_large() {
        let mut codec = HenryCodec::with_max_frame_size(10);

        // Create a message that will exceed the limit when framed
        let device_id = DeviceId::new(15).unwrap();
        let large_data = "A".repeat(100);
        let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .field(FieldData::new(large_data).unwrap())
            .build()
            .unwrap();

        let frame = Frame::from(msg);
        let framed = frame.with_framing();

        // Simulate receiving this frame
        let mut buffer = BytesMut::from(framed.as_bytes());

        let result = codec.decode(&mut buffer);
        assert!(result.is_err());

        if let Err(Error::FrameTooLarge { size, max_size }) = result {
            assert!(size > max_size);
        } else {
            panic!("Expected FrameTooLarge error");
        }
    }

    #[test]
    fn test_encode_simple_message() {
        let mut codec = HenryCodec::new();
        let mut buffer = BytesMut::new();

        let device_id = DeviceId::new(15).unwrap();
        let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();

        let result = codec.encode(msg, &mut buffer);
        assert!(result.is_ok());

        // Check framing
        assert_eq!(buffer[0], 0x02); // STX
        assert_eq!(buffer[buffer.len() - 1], 0x03); // ETX

        // Check content (without STX/ETX)
        let content = &buffer[1..buffer.len() - 1];
        assert_eq!(content, b"15+REON+RQ");
    }

    #[test]
    fn test_encode_message_with_fields() {
        let mut codec = HenryCodec::new();
        let mut buffer = BytesMut::new();

        let device_id = DeviceId::new(15).unwrap();
        let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
            .field(FieldData::new("12345678".to_string()).unwrap())
            .field(FieldData::new("20/10/2025 20:46:06".to_string()).unwrap())
            .field(FieldData::new("1".to_string()).unwrap())
            .field(FieldData::new("0".to_string()).unwrap())
            .build()
            .unwrap();

        let result = codec.encode(msg, &mut buffer);
        assert!(result.is_ok());

        // Check framing
        assert_eq!(buffer[0], 0x02); // STX
        assert_eq!(buffer[buffer.len() - 1], 0x03); // ETX

        // Verify content structure
        let content = String::from_utf8(buffer[1..buffer.len() - 1].to_vec()).unwrap();
        assert!(content.starts_with("15+REON+000+0"));
        assert!(content.contains("12345678"));
    }

    #[test]
    fn test_encode_frame_too_large() {
        let mut codec = HenryCodec::with_max_frame_size(10);
        let mut buffer = BytesMut::new();

        let device_id = DeviceId::new(15).unwrap();
        let large_data = "A".repeat(100);
        let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .field(FieldData::new(large_data).unwrap())
            .build()
            .unwrap();

        let result = codec.encode(msg, &mut buffer);
        assert!(result.is_err());

        if let Err(Error::FrameTooLarge { size, max_size }) = result {
            assert_eq!(max_size, 10);
            assert!(size > max_size);
        } else {
            panic!("Expected FrameTooLarge error");
        }
    }

    #[test]
    fn test_roundtrip_encoding_decoding() {
        let mut encoder = HenryCodec::new();
        let mut decoder = HenryCodec::new();

        // Create original message
        let device_id = DeviceId::new(15).unwrap();
        let original = MessageBuilder::new(device_id, CommandCode::AccessRequest)
            .field(FieldData::new("12345678".to_string()).unwrap())
            .field(FieldData::new("10/05/2025 12:46:06".to_string()).unwrap())
            .build()
            .unwrap();

        // Encode
        let mut buffer = BytesMut::new();
        encoder.encode(original.clone(), &mut buffer).unwrap();

        // Decode
        let decoded = decoder.decode(&mut buffer).unwrap();
        assert!(decoded.is_some());

        let msg = decoded.unwrap();
        assert_eq!(msg.device_id, original.device_id);
        assert_eq!(msg.command, original.command);
        assert_eq!(msg.fields.len(), original.fields.len());
    }

    #[test]
    fn test_encode_multiple_messages() {
        let mut codec = HenryCodec::new();
        let mut buffer = BytesMut::new();

        let device_id_1 = DeviceId::new(15).unwrap();
        let msg1 = MessageBuilder::new(device_id_1, CommandCode::QueryStatus)
            .build()
            .unwrap();

        let device_id_2 = DeviceId::new(16).unwrap();
        let msg2 = MessageBuilder::new(device_id_2, CommandCode::QueryStatus)
            .build()
            .unwrap();

        // Encode both messages
        codec.encode(msg1, &mut buffer).unwrap();
        codec.encode(msg2, &mut buffer).unwrap();

        // Buffer should contain both framed messages
        assert!(buffer.len() > 20); // Both messages with framing
    }

    #[test]
    fn test_decode_with_garbage_before_stx() {
        let mut codec = HenryCodec::new();
        // Garbage data before STX should be ignored
        let mut buffer = BytesMut::from(&b"garbage\x0215+REON+RQ\x03"[..]);

        let result = codec.decode(&mut buffer);
        assert!(result.is_ok());

        let message = result.unwrap();
        assert!(message.is_some());

        let msg = message.unwrap();
        assert_eq!(msg.device_id.as_u8(), 15);
    }
}
