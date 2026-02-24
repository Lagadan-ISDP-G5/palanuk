extern crate cu_bincode as bincode;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use propulsion_adapter::{LoopState, SteerDirection, WorkOrRestState, ZenohTopicsAdapterOutputPayload};
use cu_propulsion::WheelDirection;
use cu_cam_pan::PositionCommand;

#[derive(Reflect)]
#[reflect(no_field_bounds, from_reflect = false)]
pub struct ZSrcMerger {}

impl Freezable for ZSrcMerger {}

/// IMPORTANT: The #[serde(transparent)] is so that rmp_serde treats these tuple structs as the raw
/// types they contain, so that from_slice::<S>() in cu-zenoh-src will decode the primitive type sent from
/// the wire directly

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[derive(Reflect)]
pub struct BstnOpenLoopSpeed(pub f64);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[derive(Reflect)]
pub struct BstnLoopMode(pub u8);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[derive(Reflect)]
pub struct BstnOpenLoopDriveState(pub u8);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[derive(Reflect)]
pub struct BstnOpenLoopForcepan(pub u8);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[derive(Reflect)]
pub struct BstnOpenLoopSteerCmd(pub u8);

impl CuTask for ZSrcMerger {
    // f64 - bstn_openloop_speed
    // u8 - bstn_loopmode
    // u8 - bstn_openloop_drivestate
    // u8 - bstn_openloop_forcepan
    // u8 - bstn_openloop_steercmd

    type Input<'m>
    = input_msg!('m,
            BstnOpenLoopSpeed,
            BstnLoopMode,
            BstnOpenLoopDriveState,
            BstnOpenLoopForcepan,
            BstnOpenLoopSteerCmd
        );
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
            Some(bstn_openloop_speed),
            Some(bstn_loopmode),
            Some(bstn_openloop_drivestate),
            Some(bstn_openloop_forcepan),
            Some(bstn_openloop_steercmd)
        ) =
        (
            input.0.payload(),
            input.1.payload(),
            input.2.payload(),
            input.3.payload(),
            input.4.payload(),
        ) {
            let loop_state = match bstn_loopmode.0 {
                0 => LoopState::Open,
                1 => LoopState::Closed,
                _ => LoopState::Open
            };

            let openloop_left_speed = bstn_openloop_speed.0 as f32;
            let openloop_right_speed = bstn_openloop_speed.0 as f32;

            let drive_state = match bstn_openloop_drivestate.0 {
                0 => false, // At Rest
                1 => true, // Forward
                2 => true, // Reverse
                _ => false
            };

            // Open loop stop command overrides drivestate
            // let left_enable = true;
            let left_enable = drive_state;
            let right_enable = left_enable;

            let left_direction = match bstn_openloop_drivestate.0 {
                0 => WheelDirection::Stop,
                1 => WheelDirection::Forward,
                2 => WheelDirection::Reverse,
                _ => WheelDirection::Stop
            };

            let right_direction = left_direction;

            let steer_direction = match bstn_openloop_steercmd.0 {
                0 => SteerDirection::Free,
                1 => SteerDirection::HardLeft,
                2 => SteerDirection::HardRight,
                _ => SteerDirection::Free
            };

            let work_or_rest_state = match bstn_openloop_drivestate.0 {
                0 => WorkOrRestState::AtRest, // At Rest
                1 => WorkOrRestState::AtWork, // Forward
                2 => WorkOrRestState::AtWork, // Reverse
                _ => WorkOrRestState::AtRest
            };

            let camera_position = match bstn_openloop_forcepan.0 {
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
