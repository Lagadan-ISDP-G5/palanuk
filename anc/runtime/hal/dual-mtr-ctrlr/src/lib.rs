extern crate cu_bincode as bincode;
use cu_pid::GenericPIDTask;
use bincode::{Encode, Decode};
use serde::{Serialize, Deserialize};
use cu29::prelude::*;

pub type DualMtrCtrlr = GenericPIDTask<DualMtrCtrlrPayload>;

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
#[derive(Reflect)]
#[reflect(no_field_bounds, from_reflect = false)]
pub struct DualMtrCtrlrPayload {
    pub error: f32
}

impl From<&DualMtrCtrlrPayload> for f32 {
    fn from(payload: &DualMtrCtrlrPayload) -> f32 {
        payload.error
    }
}
