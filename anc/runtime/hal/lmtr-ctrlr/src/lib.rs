use mtr_ctrlr::*;

pub type LmtrCtrlr = Mtr<LmtrCtrlrPayload>;

pub struct LmtrCtrlrPayload {
    pub weighted_err: f32
}

impl Into<f32> for LmtrCtrlrPayload {
    fn into(self) -> f32 {
        self.weighted_err
    }
}

impl From<f32> for LmtrCtrlrPayload {
    fn from(value: f32) -> Self {
        Self { weighted_err: value }
    }
}
