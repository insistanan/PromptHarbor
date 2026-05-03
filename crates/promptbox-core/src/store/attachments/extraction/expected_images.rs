pub(in crate::store::attachments) fn expected_image_count(prompt: &str) -> usize {
    image_placeholder_count(prompt, "[Image #") + image_placeholder_count(prompt, "[图片 #")
}

pub(in crate::store::attachments) fn has_missing_images(
    expected_image_count: usize,
    captured_image_count: usize,
) -> bool {
    expected_image_count > captured_image_count
}

fn image_placeholder_count(prompt: &str, marker: &str) -> usize {
    let mut count = 0;
    let mut rest = prompt;
    while let Some(index) = rest.find(marker) {
        count += 1;
        rest = &rest[index + marker.len()..];
    }
    count
}
