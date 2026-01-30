use cu29::prelude::*;
use iceoryx2::{port::subscriber::Subscriber, prelude::*};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::ipc::{AbsLineGradientMsg, CornerDetectedMsg, CornerDirectionMsg, CornerPointMsg, HeadingErrorMsg, SERVICE_NAME_ABS_LINE_GRADIENT, SERVICE_NAME_CORNER_DETECTED, SERVICE_NAME_CORNER_DIRECTION, SERVICE_NAME_CORNER_POINT, SERVICE_NAME_HEADING_ERROR};
mod ipc;

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
    heading_error_sub: Subscriber<_, HeadingErrorMsg, ()>,
    abs_line_gradient_sub: Subscriber<_, AbsLineGradientMsg, ()>,
    corner_detected_sub: Subscriber<_, CornerDetectedMsg, ()>,
    corner_direction_sub: Subscriber<_, CornerDirectionMsg, ()>,
    corner_point_sub: Subscriber<_, CornerPointMsg, ()>
}

impl Freezable for OpenCViox2 {}

impl CuSrcTask for OpenCViox2 {
    type Output<'m> = output_msg!(OpenCViox2Payload);

    fn new(config: Option<&ComponentConfig>) -> CuResult<Self>
    where
        Self: Sized,
    {
        let heading_error_node = NodeBuilder::new().create::<ipc::Service>()?;
        let heading_error_service = heading_error_node.service_builder(SERVICE_NAME_HEADING_ERROR.try_into()?)
            .publish_subscribe::<HeadingErrorMsg>()
            .open_or_create()?;
        let heading_error_sub = heading_error_service.subscriber_builder().create()?;

        let abs_line_gradient_node = NodeBuilder::new().create::<ipc::Service>()?;
        let abs_line_gradient_service = abs_line_gradient_node.service_builder(SERVICE_NAME_ABS_LINE_GRADIENT.try_into()?)
            .publish_subscribe::<AbsLineGradientMsg>()
            .open_or_create()?;
        let abs_line_gradient_sub = abs_line_gradient_service.subscriber_builder().create()?;

        let corner_detected_node = NodeBuilder::new().create::<ipc::Service>()?;
        let corner_detected_service = corner_detected_node.service_builder(SERVICE_NAME_CORNER_DETECTED.try_into()?)
            .publish_subscribe::<CornerDetectedMsg>()
            .open_or_create()?;
        let corner_detected_sub = corner_detected_service.subscriber_builder().create()?;

        let corner_direction_node = NodeBuilder::new().create::<ipc::Service>()?;
        let corner_direction_service = corner_direction_node.service_builder(SERVICE_NAME_CORNER_DIRECTION.try_into()?)
            .publish_subscribe::<CornerDirection>()
            .open_or_create()?;
        let corner_direction_sub = corner_direction_service.subscriber_builder().create()?;

        let corner_point_node = NodeBuilder::new().create::<ipc::Service>()?;
        let corner_point_service = corner_point_node.service_builder(SERVICE_NAME_CORNER_POINT.try_into()?)
            .publish_subscribe::<CornerPointMsg>()
            .open_or_create()?;
        let corner_point_sub = corner_point_service.subscriber_builder().create()?;


        Ok(Self {
            heading_error_sub,
            abs_line_gradient_sub,
            corner_detected_sub,
            corner_direction_sub,
            corner_point_sub
        })
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
        // TODO
        Ok(())
    }
}
