extern crate cu_bincode as bincode;
use cu_pid::GenericPIDTask;
use bincode::{Encode, Decode};
use serde::{Serialize, Deserialize};

pub type LmtrSpeedCtrlr = GenericPIDTask<LmtrSpeedErrPayload>;
pub type RmtrSpeedCtrlr = GenericPIDTask<RmtrSpeedErrPayload>;

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct LmtrSpeedErrPayload {
    pub error: f32
}

impl From<&LmtrSpeedErrPayload> for f32 {
    fn from(payload: &LmtrSpeedErrPayload) -> f32 {
        payload.error
    }
}

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct RmtrSpeedErrPayload {
    pub error: f32
}

impl From<&RmtrSpeedErrPayload> for f32 {
    fn from(payload: &RmtrSpeedErrPayload) -> f32 {
        payload.error
    }
}
