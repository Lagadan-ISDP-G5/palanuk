use cu29::prelude::*;
use iceoryx2::{config::Node, port::subscriber::Subscriber, prelude::*, service::ipc::Service};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const SERVICE_NAME_HEADING_ERROR: &str = "nsm/heading_error";
pub const SERVICE_NAME_ABS_LINE_GRADIENT: &str = "nsm/abs_line_gradient";
pub const SERVICE_NAME_CORNER_DETECTED: &str = "nsm/corner_detected";
pub const SERVICE_NAME_CORNER_DIRECTION: &str = "nsm/corner_direction";
pub const SERVICE_NAME_CORNER_POINT: &str = "nsm/corner_point";


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

pub struct OpenCViox2 {
    node: Node,
    service: Service,
    subscriber: Subscriber<Service, >
}

impl Freezable for OpenCViox2 {}

impl CuSrcTask for OpenCViox2 {
    type Output<'m> = output_msg!(OpenCViox2Payload);

    fn new(config: Option<&ComponentConfig>) -> CuResult<Self>
    where
        Self: Sized,
    {
        
        Ok(Self {})
    }

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        // TODO
        Ok(())
    }

    fn process(&mut self, _clock: &RobotClock, output: &mut Self::Output<'_>) -> CuResult<()> {
        // TODO
        Ok(())
    }

    fn stop(&mut self, _clock: &RobotClock) -> CuResult<()> {
        Ok(())
    }
}
