use cu29::prelude::*;
use iceoryx2::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub enum CornerDirection {
    Left,
    Right // only Right corners exist in the track
}

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct OpenCViox2Payload {
    pub abs_line_gradient: f32,
    pub heading_error: f32,
    pub corner_detected: bool,
    pub corner_coords: (f32, f32),
    pub corner_direction: CornerDirection,
}
