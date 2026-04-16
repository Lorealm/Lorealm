//! PNG character-card parser.
//!
//! Reads and writes character cards embedded in PNG `tEXt` chunks with
//! minimal allocations. Parsing performs zero copies of chunk payloads;
//! the only allocations per call are:
//!
//!   1. The `Vec<PngChunk<'a>>` spine (~10 entries, a few hundred bytes).
//!   2. The base64-decoded JSON bytes (unavoidable — decoding allocates).
//!   3. The final stripped/embedded PNG buffer (the return value).
//!
//! The parser is deliberately decoupled from the domain types. It returns
//! raw JSON bytes plus a detected-spec tag; conversion into
//! `Character`/`EldoriaCard`/etc. happens in the service layer.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::Deserialize;

use crate::errors::CardParseError;

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub type Result<T> = std::result::Result<T, CardParseError>;

// -----------------------------------------------------------------------------
// Public types
// -----------------------------------------------------------------------------

/// Result of parsing a character card PNG.
#[derive(Debug)]
pub struct ParsedCharacterCard {
    /// Raw JSON bytes, base64-decoded. The caller picks a deserialization
    /// target based on `detected_spec`.
    pub json_bytes: Vec<u8>,

    /// The original PNG with the card's tEXt chunk removed. Persist this
    /// if you want to store a clean image alongside the parsed character.
    pub stripped_image: Vec<u8>,

    /// What the card's top-level `spec` field reported.
    pub detected_spec: DetectedSpec,
}

/// Identifies which card format was detected in the JSON's `spec` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedSpec {
    /// SillyTavern-compatible CCv2 (`"chara_card_v2"`).
    CharaV2,
    /// Any other value encountered in the `spec` field. Preserved verbatim.
    Unknown(String),
}

// -----------------------------------------------------------------------------
// Public API — read
// -----------------------------------------------------------------------------

/// Parse a character card PNG and return the decoded JSON, a stripped
/// image, and a detected-spec tag.
///
/// `identifier` is the tEXt keyword to search for. CCv2 cards use `"chara"`.
pub fn parse_character_card_png(png_data: &[u8], identifier: &str) -> Result<ParsedCharacterCard> {
    let chunks = parse_png_chunks(png_data)?;

    let (chunk_idx, base64_payload) = find_card_chunk(&chunks, identifier.as_bytes())
        .ok_or_else(|| CardParseError::ChunkNotFound(identifier.to_string()))?;

    let json_bytes = BASE64.decode(base64_payload)?;
    let detected_spec = peek_spec(&json_bytes)?;
    let stripped_image = rebuild_png_excluding(&chunks, chunk_idx);

    Ok(ParsedCharacterCard {
        json_bytes,
        stripped_image,
        detected_spec,
    })
}

// -----------------------------------------------------------------------------
// Public API — write
// -----------------------------------------------------------------------------

/// Embed character JSON into a PNG as a tEXt chunk, inserted just before IEND.
///
/// The caller provides `json_bytes` (already serialized) so this function
/// stays free of domain-type dependencies.
pub fn embed_character_card_png(
    image_data: &[u8],
    json_bytes: &[u8],
    identifier: &str,
) -> Result<Vec<u8>> {
    let chunks = parse_png_chunks(image_data)?;

    let iend_idx = chunks
        .iter()
        .position(|c| &c.chunk_type == b"IEND")
        .ok_or(CardParseError::InvalidChunkFormat)?;

    // Build the tEXt payload: <keyword>\0<base64-of-json>.
    let id_bytes = identifier.as_bytes();
    let encoded_len =
        base64::encoded_len(json_bytes.len(), true).ok_or(CardParseError::InvalidChunkFormat)?;
    let payload_len = id_bytes.len() + 1 + encoded_len;

    let mut payload = Vec::with_capacity(payload_len);
    payload.extend_from_slice(id_bytes);
    payload.push(0);
    let base_offset = payload.len();
    payload.resize(base_offset + encoded_len, 0);
    let written = BASE64.encode_slice(json_bytes, &mut payload[base_offset..])?;
    debug_assert_eq!(written, encoded_len);

    // CRC covers chunk_type + payload. Stream them into the hasher; avoid
    // a concat allocation.
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(b"tEXt");
    hasher.update(&payload);
    let crc = hasher.finalize();

    // Exact output size = signature + all original chunks (each 12 + data) +
    // our new chunk (12 + payload).
    let existing_size: usize = chunks.iter().map(|c| 12 + c.data.len()).sum();
    let total = PNG_SIGNATURE.len() + existing_size + 12 + payload.len();

    let mut output = Vec::with_capacity(total);
    output.extend_from_slice(&PNG_SIGNATURE);

    for (i, chunk) in chunks.iter().enumerate() {
        if i == iend_idx {
            // Insert new tEXt chunk immediately before IEND.
            output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            output.extend_from_slice(b"tEXt");
            output.extend_from_slice(&payload);
            output.extend_from_slice(&crc.to_be_bytes());
        }
        write_chunk(&mut output, chunk);
    }

    debug_assert_eq!(output.len(), total);
    Ok(output)
}

// -----------------------------------------------------------------------------
// Internals
// -----------------------------------------------------------------------------

/// Borrowed view of a PNG chunk. All payload data points into the original
/// input buffer — no copies.
struct PngChunk<'a> {
    chunk_type: [u8; 4],
    data: &'a [u8],
    /// Original CRC bytes from the input. For chunks we pass through
    /// unchanged, reusing these bytes means we never recompute CRC.
    crc_bytes: &'a [u8; 4],
}

/// Parse a PNG into a list of borrowed chunks. Stops after IEND.
fn parse_png_chunks(data: &[u8]) -> Result<Vec<PngChunk<'_>>> {
    if data.len() < 8 || data[..8] != PNG_SIGNATURE {
        return Err(CardParseError::InvalidPng);
    }

    let mut chunks = Vec::with_capacity(16);
    let mut pos = 8;

    while pos + 12 <= data.len() {
        let length =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        let chunk_type = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
        pos += 4;

        let payload_end = pos
            .checked_add(length)
            .ok_or(CardParseError::InvalidChunkFormat)?;
        if payload_end + 4 > data.len() {
            return Err(CardParseError::InvalidChunkFormat);
        }

        let chunk_data = &data[pos..payload_end];
        pos = payload_end;

        // Borrow the 4 CRC bytes as a fixed-size array reference.
        let crc_bytes: &[u8; 4] = (&data[pos..pos + 4]).try_into().unwrap();
        pos += 4;

        chunks.push(PngChunk {
            chunk_type,
            data: chunk_data,
            crc_bytes,
        });

        if &chunk_type == b"IEND" {
            break;
        }
    }

    Ok(chunks)
}

/// Scan chunks for the first `tEXt` whose keyword matches `identifier`.
/// Returns the chunk index and the base64 payload slice (everything after
/// the null separator).
fn find_card_chunk<'a>(chunks: &'a [PngChunk<'a>], identifier: &[u8]) -> Option<(usize, &'a [u8])> {
    for (i, chunk) in chunks.iter().enumerate() {
        if &chunk.chunk_type != b"tEXt" {
            continue;
        }
        if !chunk.data.starts_with(identifier) {
            continue;
        }
        if chunk.data.get(identifier.len()) != Some(&0) {
            continue;
        }
        let payload = &chunk.data[identifier.len() + 1..];
        return Some((i, payload));
    }
    None
}

/// Cheap peek at the `spec` field. Uses a minimal struct that borrows
/// the string from the input buffer — no full JSON deserialization.
fn peek_spec(json_bytes: &[u8]) -> Result<DetectedSpec> {
    #[derive(Deserialize)]
    struct SpecPeek<'a> {
        #[serde(borrow, default)]
        spec: Option<std::borrow::Cow<'a, str>>,
    }

    let peek: SpecPeek = serde_json::from_slice(json_bytes)?;

    Ok(match peek.spec.as_deref() {
        Some("chara_card_v2") => DetectedSpec::CharaV2,
        Some(other) => DetectedSpec::Unknown(other.to_string()),
        None => DetectedSpec::Unknown(String::new()),
    })
}

/// Rebuild the PNG byte-stream, skipping the chunk at `exclude_idx`.
/// Exact-sized single allocation.
fn rebuild_png_excluding(chunks: &[PngChunk<'_>], exclude_idx: usize) -> Vec<u8> {
    let total = PNG_SIGNATURE.len()
        + chunks
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != exclude_idx)
            .map(|(_, c)| 12 + c.data.len())
            .sum::<usize>();

    let mut output = Vec::with_capacity(total);
    output.extend_from_slice(&PNG_SIGNATURE);

    for (i, chunk) in chunks.iter().enumerate() {
        if i == exclude_idx {
            continue;
        }
        write_chunk(&mut output, chunk);
    }

    debug_assert_eq!(output.len(), total);
    output
}

/// Write a chunk to a pre-sized buffer. The buffer must have capacity for
/// `12 + chunk.data.len()` more bytes; `extend_from_slice` will not
/// reallocate under that precondition.
#[inline]
fn write_chunk(output: &mut Vec<u8>, chunk: &PngChunk<'_>) {
    output.extend_from_slice(&(chunk.data.len() as u32).to_be_bytes());
    output.extend_from_slice(&chunk.chunk_type);
    output.extend_from_slice(chunk.data);
    output.extend_from_slice(chunk.crc_bytes);
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid PNG: 1x1 black pixel, three chunks (IHDR/IDAT/IEND).
    fn minimal_png() -> Vec<u8> {
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR length
            0x49, 0x48, 0x44, 0x52, // "IHDR"
            0x00, 0x00, 0x00, 0x01, // Width: 1
            0x00, 0x00, 0x00, 0x01, // Height: 1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // CRC
            0x00, 0x00, 0x00, 0x0C, // IDAT length
            0x49, 0x44, 0x41, 0x54, // "IDAT"
            0x08, 0xD7, 0x63, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0x27, 0x9B,
            0x4D, 0x7E, // CRC
            0x00, 0x00, 0x00, 0x00, // IEND length
            0x49, 0x45, 0x4E, 0x44, // "IEND"
            0xAE, 0x42, 0x60, 0x82, // CRC
        ]
    }

    #[test]
    fn parses_three_chunks_from_minimal_png() {
        let png = minimal_png();
        let chunks = parse_png_chunks(&png).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(&chunks[0].chunk_type, b"IHDR");
        assert_eq!(&chunks[1].chunk_type, b"IDAT");
        assert_eq!(&chunks[2].chunk_type, b"IEND");
    }

    #[test]
    fn rebuild_is_byte_exact_with_no_exclusion() {
        // exclude_idx past the end = no exclusion.
        let png = minimal_png();
        let chunks = parse_png_chunks(&png).unwrap();
        let rebuilt = rebuild_png_excluding(&chunks, usize::MAX);
        assert_eq!(png, rebuilt);
    }

    #[test]
    fn embed_then_parse_round_trip() {
        let png = minimal_png();
        let json = br#"{"spec":"chara_card_v2","spec_version":"2.0","data":{"name":"Test"}}"#;

        let embedded = embed_character_card_png(&png, json, "chara").unwrap();
        let parsed = parse_character_card_png(&embedded, "chara").unwrap();

        assert_eq!(parsed.json_bytes, json);
        assert_eq!(parsed.detected_spec, DetectedSpec::CharaV2);
        // After stripping, we should be back to the original PNG.
        assert_eq!(parsed.stripped_image, png);
    }

    #[test]
    fn unknown_spec_preserves_value() {
        let png = minimal_png();
        let json = br#"{"spec":"something_else","data":{}}"#;
        let embedded = embed_character_card_png(&png, json, "chara").unwrap();
        let parsed = parse_character_card_png(&embedded, "chara").unwrap();
        assert_eq!(
            parsed.detected_spec,
            DetectedSpec::Unknown("something_else".to_string())
        );
    }

    #[test]
    fn rejects_non_png() {
        let not_png = b"this is not a png";
        let err = parse_character_card_png(not_png, "chara").unwrap_err();
        assert!(matches!(err, CardParseError::InvalidPng));
    }

    #[test]
    fn errors_when_chunk_missing() {
        let png = minimal_png();
        let err = parse_character_card_png(&png, "chara").unwrap_err();
        assert!(matches!(err, CardParseError::ChunkNotFound(_)));
    }

    #[test]
    fn different_identifier_does_not_match() {
        let png = minimal_png();
        let json = br#"{"spec":"chara_card_v2","data":{}}"#;
        let embedded = embed_character_card_png(&png, json, "chara").unwrap();
        let err = parse_character_card_png(&embedded, "ccv3").unwrap_err();
        assert!(matches!(err, CardParseError::ChunkNotFound(_)));
    }
}
