use mtr_ctrlr::*;

pub type RmtrCtrlr = Mtr<RmtrCtrlrPayload>;

pub struct RmtrCtrlrPayload {
    pub weighted_err: f32
}

impl Into<f32> for RmtrCtrlrPayload {
    fn into(self) -> f32 {
        self.weighted_err
    }
}

impl From<f32> for RmtrCtrlrPayload {
    fn from(value: f32) -> Self {
        Self { weighted_err: value }
    }
}
