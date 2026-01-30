use cu29::prelude::*;
use iceoryx2::prelude::*;
use iceoryx2::prelude::ipc::Service;
use iceoryx2::port::subscriber::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::ipc::{AbsLineGradientMsg, CornerDetectedMsg, CornerDirectionMsg, CornerPointMsg, HeadingErrorMsg, SERVICE_NAME_ABS_LINE_GRADIENT, SERVICE_NAME_CORNER_DETECTED, SERVICE_NAME_CORNER_DIRECTION, SERVICE_NAME_CORNER_POINT, SERVICE_NAME_HEADING_ERROR};
mod ipc;

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub enum CornerDirection {
    Left,
    #[default]
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
    heading_error_sub: Subscriber<Service, HeadingErrorMsg, ()>,
    abs_line_gradient_sub: Subscriber<Service, AbsLineGradientMsg, ()>,
    corner_detected_sub: Subscriber<Service, CornerDetectedMsg, ()>,
    corner_direction_sub: Subscriber<Service, CornerDirectionMsg, ()>,
    corner_point_sub: Subscriber<Service, CornerPointMsg, ()>
}

impl Freezable for OpenCViox2 {}

impl CuSrcTask for OpenCViox2 {
    type Output<'m> = output_msg!(OpenCViox2Payload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where
        Self: Sized,
    {
        let heading_error_node = NodeBuilder::new().create::<Service>().map_err(|_| -> CuError {CuError::from("build node failed")})?;
        let heading_error_service = heading_error_node.service_builder(&ServiceName::new(SERVICE_NAME_HEADING_ERROR).unwrap())
            .publish_subscribe::<HeadingErrorMsg>()
            .open_or_create().map_err(|_| -> CuError {CuError::from("build service failed")})?;
        let heading_error_sub = heading_error_service.subscriber_builder().create().map_err(|_| -> CuError {CuError::from("build sub failed")})?;


        let abs_line_gradient_node = NodeBuilder::new().create::<Service>().map_err(|_| -> CuError {CuError::from("build node failed")})?;
        let abs_line_gradient_service = abs_line_gradient_node.service_builder(&ServiceName::new(SERVICE_NAME_ABS_LINE_GRADIENT).unwrap())
            .publish_subscribe::<AbsLineGradientMsg>()
            .open_or_create().map_err(|_| -> CuError {CuError::from("build service failed")})?;
        let abs_line_gradient_sub = abs_line_gradient_service.subscriber_builder().create().map_err(|_| -> CuError {CuError::from("build sub failed")})?;


        let corner_detected_node = NodeBuilder::new().create::<Service>().map_err(|_| -> CuError {CuError::from("build node failed")})?;
        let corner_detected_service = corner_detected_node.service_builder(&ServiceName::new(SERVICE_NAME_CORNER_DETECTED).unwrap())
            .publish_subscribe::<CornerDetectedMsg>()
            .open_or_create().map_err(|_| -> CuError {CuError::from("build service failed")})?;
        let corner_detected_sub = corner_detected_service.subscriber_builder().create().map_err(|_| -> CuError {CuError::from("build sub failed")})?;


        let corner_direction_node = NodeBuilder::new().create::<Service>().map_err(|_| -> CuError {CuError::from("build node failed")})?;
        let corner_direction_service = corner_direction_node.service_builder(&ServiceName::new(SERVICE_NAME_CORNER_DIRECTION).unwrap())
            .publish_subscribe::<CornerDirectionMsg>()
            .open_or_create().map_err(|_| -> CuError {CuError::from("build service failed")})?;
        let corner_direction_sub = corner_direction_service.subscriber_builder().create().map_err(|_| -> CuError {CuError::from("build sub failed")})?;


        let corner_point_node = NodeBuilder::new().create::<Service>().map_err(|_| -> CuError {CuError::from("build node failed")})?;
        let corner_point_service = corner_point_node.service_builder(&ServiceName::new(SERVICE_NAME_CORNER_POINT).unwrap())
            .publish_subscribe::<CornerPointMsg>()
            .open_or_create().map_err(|_| -> CuError {CuError::from("build service failed")})?;
        let corner_point_sub = corner_point_service.subscriber_builder().create().map_err(|_| -> CuError {CuError::from("build sub failed")})?;

        Ok(Self {
            heading_error_sub,
            abs_line_gradient_sub,
            corner_detected_sub,
            corner_direction_sub,
            corner_point_sub
        })
    }

    fn process(&mut self, _clock: &RobotClock, output: &mut Self::Output<'_>) -> CuResult<()> {
        if let (
            Some(heading_error_sub),
            Some(abs_line_gradient_sub),
            Some(corner_detected_sub),
            Some(corner_direction_sub),
            Some(corner_point_sub)
        )
        = (
            self.heading_error_sub.receive().map_err(|_| -> CuError {CuError::from("iox2 recv failed")})?,
            self.abs_line_gradient_sub.receive().map_err(|_| -> CuError {CuError::from("iox2 recv failed")})?,
            self.corner_detected_sub.receive().map_err(|_| -> CuError {CuError::from("iox2 recv failed")})?,
            self.corner_direction_sub.receive().map_err(|_| -> CuError {CuError::from("iox2 recv failed")})?,
            self.corner_point_sub.receive().map_err(|_| -> CuError {CuError::from("iox2 recv failed")})?
        )

        {
            let abs_line_gradient = *&abs_line_gradient_sub.payload().value;
            let heading_error = *&heading_error_sub.payload().value;

            let corner_detected = match corner_detected_sub.payload().detected {
                0 => false,
                1 => true,
                _ => false
            };

            let corner_coords = (corner_point_sub.payload().x, corner_point_sub.payload().y);

            let corner_direction;
            if corner_direction_sub.payload().x.signum() == 1.0 {
                corner_direction = CornerDirection::Right;
            }
            else {
                corner_direction = CornerDirection::Left; // basically unreachable
            }
            output.set_payload(OpenCViox2Payload { abs_line_gradient, heading_error, corner_detected, corner_coords, corner_direction });
        }

        Ok(())
    }

}
