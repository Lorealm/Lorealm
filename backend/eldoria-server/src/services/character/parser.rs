use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use thiserror::Error;
use uuid::Uuid;

use crate::services::character::CharaV2;
use crate::services::character::EldoriaCharacter;

#[derive(Error, Debug)]
pub enum CharacterCardError {
    #[error("Invalid PNG format")]
    InvalidPng,
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    
    #[error("Character card chunk not found with identifier: {0}")]
    ChunkNotFound(String),
    
    #[error("Unsupported spec: {0} version {1}")]
    UnsupportedSpec(String, String),
    
    #[error("Invalid chunk data format")]
    InvalidChunkFormat,
}

pub type Result<T> = std::result::Result<T, CharacterCardError>;

pub const CHARA_V2_SPEC: &str = "chara_card_v2";
pub const ELDORIA_CHARACTER_SPEC: &str = "eldoria_chara";

const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Debug, Clone)]
pub struct ParsedCharacterCard {
    pub character: EldoriaCharacter,
    pub stripped_image: Vec<u8>,
}

struct PngChunk {
    length: u32,
    chunk_type: [u8; 4],
    data: Vec<u8>,
    crc: u32,
}

#[cfg(test)]
impl PngChunk {
    fn type_str(&self) -> String {
        String::from_utf8_lossy(&self.chunk_type).to_string()
    }
}

/// Fast PNG chunk parser that works directly with bytes
fn parse_png_chunks(data: &[u8]) -> Result<Vec<PngChunk>> {
    if data.len() < 8 || &data[0..8] != PNG_SIGNATURE {
        return Err(CharacterCardError::InvalidPng);
    }

    let mut chunks = Vec::new();
    let mut pos = 8; // Skip PNG signature

    while pos + 12 <= data.len() {
        // Read chunk length (4 bytes)
        let length = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        // Read chunk type (4 bytes)
        let chunk_type = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
        pos += 4;

        // Check if we have enough data
        if pos + length as usize + 4 > data.len() {
            return Err(CharacterCardError::InvalidChunkFormat);
        }

        // Read chunk data
        let chunk_data = data[pos..pos + length as usize].to_vec();
        pos += length as usize;

        // Read CRC (4 bytes)
        let crc = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        chunks.push(PngChunk {
            length,
            chunk_type,
            data: chunk_data,
            crc,
        });

        // Stop at IEND
        if &chunk_type == b"IEND" {
            break;
        }
    }

    Ok(chunks)
}

/// Rebuild PNG from chunks
fn rebuild_png_from_chunks(chunks: &[PngChunk]) -> Vec<u8> {
    let mut output = Vec::with_capacity(8 + chunks.len() * 100); // Estimate size
    
    // Write PNG signature
    output.extend_from_slice(PNG_SIGNATURE);
    
    // Write all chunks
    for chunk in chunks {
        output.extend_from_slice(&chunk.length.to_be_bytes());
        output.extend_from_slice(&chunk.chunk_type);
        output.extend_from_slice(&chunk.data);
        output.extend_from_slice(&chunk.crc.to_be_bytes());
    }
    
    output
}

/// Extract and parse character card data from a PNG file.
pub fn parse_character_card(png_data: &[u8], identifier: &str, owner: Uuid) -> Result<ParsedCharacterCard> {
    // Parse PNG chunks directly without decoding the image
    let mut chunks = parse_png_chunks(png_data)?;
    
    // Find and extract the tEXt chunk with our identifier
    let identifier_bytes = identifier.as_bytes();
    let mut text_data = None;
    let mut chunk_index = None;
    
    for (i, chunk) in chunks.iter().enumerate() {
        if &chunk.chunk_type == b"tEXt" {
            // Check if this tEXt chunk starts with our identifier
            if chunk.data.len() > identifier_bytes.len()
                && chunk.data[..identifier_bytes.len()] == *identifier_bytes
                && chunk.data[identifier_bytes.len()] == 0
            {
                // Extract the data after identifier and null byte
                let data_start = identifier_bytes.len() + 1;
                text_data = Some(&chunk.data[data_start..]);
                chunk_index = Some(i);
                break;
            }
        }
    }
    
    let text_data = text_data.ok_or_else(|| {
        CharacterCardError::ChunkNotFound(identifier.to_string())
    })?;
    
    // Convert to string and decode base64
    let text_str = std::str::from_utf8(text_data)?;
    let decoded = BASE64_STANDARD.decode(text_str)?;
    
    // Parse JSON to determine spec
    let json_value: serde_json::Value = serde_json::from_slice(&decoded)?;
    
    // Determine spec and version
    let spec = json_value
        .get("spec")
        .and_then(|v| v.as_str())
        .unwrap_or(CHARA_V2_SPEC);
    
    let spec_version = json_value
        .get("spec_version")
        .or_else(|| json_value.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("2.0");
    
    // Parse into appropriate format and convert to EldoriaCharacter
    let character = match spec {
        CHARA_V2_SPEC => {
            let mut chara_v2: CharaV2 = serde_json::from_value(json_value)?;
            chara_v2.user_id = owner;
            EldoriaCharacter::from(chara_v2)
        }
        ELDORIA_CHARACTER_SPEC => {
            serde_json::from_value(json_value)?
        }
        _ => {
            return Err(CharacterCardError::UnsupportedSpec(
                spec.to_string(),
                spec_version.to_string(),
            ));
        }
    };
    
    // Remove the tEXt chunk and rebuild PNG
    if let Some(idx) = chunk_index {
        chunks.remove(idx);
    }
    let stripped_image = rebuild_png_from_chunks(&chunks);
    
    Ok(ParsedCharacterCard {
        character,
        stripped_image,
    })
}

/// Embed character data into a PNG image as a tEXt chunk.
pub fn create_character_card_png(
    image_data: &[u8],
    character: &EldoriaCharacter,
    identifier: &str,
) -> Result<Vec<u8>> {
    let json = serde_json::to_string(character)?;
    
    let encoded = BASE64_STANDARD.encode(json.as_bytes());
    
    // Parse existing chunks
    let mut chunks = parse_png_chunks(image_data)?;
    
    // Find IEND chunk position
    let iend_pos = chunks
        .iter()
        .position(|c| &c.chunk_type == b"IEND")
        .ok_or(CharacterCardError::InvalidChunkFormat)?;
    
    // Create tEXt chunk data
    let mut chunk_data = Vec::new();
    chunk_data.extend_from_slice(identifier.as_bytes());
    chunk_data.push(0); // Null separator
    chunk_data.extend_from_slice(encoded.as_bytes());
    
    // Calculate CRC for tEXt chunk
    let mut crc_data = Vec::new();
    crc_data.extend_from_slice(b"tEXt");
    crc_data.extend_from_slice(&chunk_data);
    let crc = crc32fast::hash(&crc_data);
    
    // Create the chunk
    let text_chunk = PngChunk {
        length: chunk_data.len() as u32,
        chunk_type: *b"tEXt",
        data: chunk_data,
        crc,
    };
    
    // Insert before IEND
    chunks.insert(iend_pos, text_chunk);
    
    Ok(rebuild_png_from_chunks(&chunks))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_png() -> Vec<u8> {
        // A minimal valid PNG (1x1 black pixel)
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR length
            0x49, 0x48, 0x44, 0x52, // "IHDR"
            0x00, 0x00, 0x00, 0x01, // Width: 1
            0x00, 0x00, 0x00, 0x01, // Height: 1
            0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, etc.
            0x90, 0x77, 0x53, 0xDE, // CRC
            0x00, 0x00, 0x00, 0x0C, // IDAT length
            0x49, 0x44, 0x41, 0x54, // "IDAT"
            0x08, 0xD7, 0x63, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01,
            0x27, 0x9B, 0x4D, 0x7E, // CRC
            0x00, 0x00, 0x00, 0x00, // IEND length
            0x49, 0x45, 0x4E, 0x44, // "IEND"
            0xAE, 0x42, 0x60, 0x82, // CRC
        ]
    }

    #[test]
    fn test_parse_png_chunks() {
        let png = create_minimal_png();
        let chunks = parse_png_chunks(&png).unwrap();
        
        assert_eq!(chunks.len(), 3); // IHDR, IDAT, IEND
        assert_eq!(chunks[0].type_str(), "IHDR");
        assert_eq!(chunks[1].type_str(), "IDAT");
        assert_eq!(chunks[2].type_str(), "IEND");
    }

    #[test]
    fn test_rebuild_png() {
        let original = create_minimal_png();
        let chunks = parse_png_chunks(&original).unwrap();
        let rebuilt = rebuild_png_from_chunks(&chunks);
        
        // Should be identical
        assert_eq!(original, rebuilt);
    }
}