use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};

pub(super) fn decode_image_data_url(value: &str) -> Option<(String, Vec<u8>)> {
    let (metadata, data) = value.split_once(',')?;
    let metadata = metadata.trim();
    if !metadata.starts_with("data:") || !metadata.ends_with(";base64") {
        return None;
    }

    let mime_type = metadata
        .trim_start_matches("data:")
        .trim_end_matches(";base64")
        .split(';')
        .next()
        .unwrap_or("image/png")
        .trim();
    decode_base64_image(mime_type, data)
}

pub(super) fn decode_base64_image(mime_type: &str, data: &str) -> Option<(String, Vec<u8>)> {
    let mime_type = normalize_image_mime_type(mime_type);
    if !mime_type.starts_with("image/") {
        return None;
    }
    let normalized_data = data
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let bytes = BASE64_STANDARD.decode(normalized_data).ok()?;
    if bytes.is_empty() {
        return None;
    }

    Some((mime_type, bytes))
}

fn normalize_image_mime_type(value: &str) -> String {
    let normalized = value
        .trim()
        .split(';')
        .next()
        .unwrap_or("image/png")
        .to_ascii_lowercase();
    if normalized.starts_with("image/") {
        normalized
    } else {
        "image/png".to_string()
    }
}
