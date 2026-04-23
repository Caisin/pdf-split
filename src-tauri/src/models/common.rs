use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImageBytesResult {
    pub bytes: Vec<u8>,
}

pub(super) fn default_slanted_watermark_rotation_degrees() -> f32 {
    -1.0_f32.to_degrees()
}
