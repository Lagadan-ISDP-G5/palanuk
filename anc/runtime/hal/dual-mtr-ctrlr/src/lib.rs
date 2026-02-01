extern crate cu_bincode as bincode;
use cu_pid::GenericPIDTask;
use bincode::{Encode, Decode};
use serde::{Serialize, Deserialize};

pub type DualMtrCtrlr = GenericPIDTask<DualMtrCtrlrPayload>;

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct DualMtrCtrlrPayload {
    pub error: f32
}

impl From<&DualMtrCtrlrPayload> for f32 {
    fn from(payload: &DualMtrCtrlrPayload) -> f32 {
        payload.error
    }
}
