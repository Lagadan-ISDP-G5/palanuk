extern crate cu_bincode as bincode;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use propulsion_adapter::{LoopState, SteerDirection, WorkOrRestState, ZenohTopicsAdapterOutputPayload};
use cu_propulsion::WheelDirection;
use cu_cam_pan::PositionCommand;

pub struct ZSrcMerger {}

impl Freezable for ZSrcMerger {}

impl CuTask for ZSrcMerger {
    // f64 - odd_openloop_speed
    // u8 - odd_openloop_stop
    // u8 - odd_loopmode
    // u8 - odd_openloop_drivestate
    // u8 - odd_openloop_forcepan

    type Input<'m> = input_msg!('m, f64, u8, u8, u8, u8);
    type Output<'m> = output_msg!(ZenohTopicsAdapterOutputPayload);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
        where
            Self: Sized {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        if let (
            Some(odd_openloop_speed),
            Some(odd_openloop_stop),
            Some(odd_loopmode),
            Some(odd_openloop_drivestate),
            Some(odd_openloop_forcepan)
        ) =
        (
            input.0.payload(),
            input.1.payload(),
            input.2.payload(),
            input.3.payload(),
            input.4.payload()
        ) {
            let loop_state = match *odd_loopmode {
                0 => LoopState::Closed,
                1 => LoopState::Open,
                _ => LoopState::Closed
            };

            let openloop_left_speed = *odd_openloop_speed as f32;
            let openloop_right_speed = *odd_openloop_speed as f32;

            let left_enable = match *odd_openloop_drivestate {
                0 => false, // At Rest
                1 => true, // Forward
                2 => true, // Reverse
                _ => false
            };

            let right_enable = left_enable;

            let left_direction = match *odd_openloop_drivestate {
                0 => WheelDirection::Stop,
                1 => WheelDirection::Forward,
                2 => WheelDirection::Reverse,
                _ => WheelDirection::Stop
            };

            let right_direction = left_direction;

            let steer_direction = SteerDirection::Center;

            let work_or_rest_state = match *odd_openloop_drivestate {
                0 => WorkOrRestState::AtRest, // At Rest
                1 => WorkOrRestState::AtWork, // Forward
                2 => WorkOrRestState::AtWork, // Reverse
                _ => WorkOrRestState::AtRest
            };

            let camera_position = match *odd_openloop_forcepan {
                0 => PositionCommand::Front,
                1 => PositionCommand::Left,
                2 => PositionCommand::Right,
                _ => PositionCommand::Front
            };

            output.set_payload(
                ZenohTopicsAdapterOutputPayload {
                    loop_state,
                    left_enable,
                    right_enable,
                    openloop_left_speed,
                    openloop_right_speed,
                    left_direction,
                    right_direction,
                    steer_direction,
                    work_or_rest_state,
                    camera_position
                }
            );

        }

        Ok(())
    }

}
