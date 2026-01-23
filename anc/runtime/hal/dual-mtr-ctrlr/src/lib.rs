use mtr_ctrlr::*;

pub type DualMtrCtrlr = Mtr<DualMtrCtrlrPayload>;

pub struct DualMtrCtrlrPayload {
    pub weighted_err: f32
}

impl Into<f32> for DualMtrCtrlrPayload {
    fn into(self) -> f32 {
        self.weighted_err
    }
}

impl From<f32> for DualMtrCtrlrPayload {
    fn from(value: f32) -> Self {
        Self { weighted_err: value }
    }
}
