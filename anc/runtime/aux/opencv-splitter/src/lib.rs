
/// we need this dedicated splitter to prevent copper from instantiating multiple of the same iox2 publishers

extern crate cu_bincode as bincode;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use opencv_iox2::OpenCViox2Payload;

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct NsmPayload {
    pub abs_line_gradient: f32,
    pub heading_error: f32,
    pub corner_detected: bool,
    pub corner_coords: (f32, f32),
    pub corner_direction: opencv_iox2::CornerDirection,
    pub vertical_line_valid: bool
}

impl From<&OpenCViox2Payload> for NsmPayload {
    fn from(value: &OpenCViox2Payload) -> Self {
        Self {
            abs_line_gradient: value.abs_line_gradient,
            heading_error: value.heading_error,
            corner_detected: value.corner_detected,
            corner_coords: value.corner_coords,
            corner_direction: value.corner_direction,
            vertical_line_valid: value.vertical_line_valid
        }
    }
}

pub struct OpenCvSplitter {
    last_value: Option<NsmPayload>,
}

impl Freezable for OpenCvSplitter {}

impl CuTask for OpenCvSplitter {
    type Input<'m> = input_msg!('m, OpenCViox2Payload);
    type Output<'m> = output_msg!(NsmPayload);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where
        Self: Sized,
    {
        Ok(Self { last_value: None })
    }

    fn process(
        &mut self,
        _clock: &RobotClock,
        input: &Self::Input<'_>,
        output: &mut Self::Output<'_>,
    ) -> CuResult<()> {

        if let Some(opencv_msg) = input.payload() {
            self.last_value = Some(NsmPayload::from(opencv_msg));
        }

        if let Some(value) = self.last_value {
            output.set_payload(value);
        }

        Ok(())
    }
}
