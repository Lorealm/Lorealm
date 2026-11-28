/// The MIME type of `bytes`
pub fn image_type(bytes: &[u8]) -> Option<&'static str> {
    let mime = infer::get(bytes)?.mime_type();
    matches!(
        mime,
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    )
    .then_some(mime)
}
