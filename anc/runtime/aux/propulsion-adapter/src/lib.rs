extern crate cu_bincode as bincode;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use cu_cam_pan::{CameraPanningPayload, PositionCommand};
use cu_propulsion::{PropulsionPayload, WheelDirection};
use cu_hcsr04::{HcSr04Payload};
use opencv_iox2::OpenCViox2Payload;

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub enum LoopState {
    #[default]
    Closed,
    Open
}

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub enum WorkOrRestState {
    #[default]
    AtRest,
    AtWork
}

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub enum SteerDirection {
    #[default]
    Center,
    HardRight,
    SlightRight,
    HardLeft,
    SlightLeft
}

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct PropulsionAdapterOutputPayload {
    pub loop_state: LoopState,
    pub propulsion_payload: PropulsionPayload,
    pub panner_payload: CameraPanningPayload,
    pub weighted_error: f32,
    pub is_e_stop_triggered: bool,
    pub distance: f64,
}

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct ZenohTopicsAdapterOutputPayload {
    pub loop_state: LoopState,
    pub left_enable: bool,
    pub right_enable: bool,
    pub openloop_left_speed: f32,
    pub openloop_right_speed: f32,
    pub left_direction: WheelDirection,
    pub right_direction: WheelDirection,
    pub steer_direction: SteerDirection,
    pub work_or_rest_state: WorkOrRestState,
    pub camera_position: PositionCommand,
}

pub struct PropulsionAdapter {
    e_stop_threshold_cm: f64
}

impl Freezable for PropulsionAdapter {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.e_stop_threshold_cm, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.e_stop_threshold_cm = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuTask for PropulsionAdapter {
    type Input<'m> = input_msg!('m, ZenohTopicsAdapterOutputPayload, HcSr04Payload, OpenCViox2Payload);
    type Output<'m> = output_msg!(PropulsionAdapterOutputPayload);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for PropulsionAdapter in RON")?;

        let e_stop_threshold_cm: f64 = kv
            .get("e_stop_threshold_cm")
            .expect("e_stop_threshold_cm for PropulsionAdapter not set in RON config.")
            .clone()
            .into();

        Ok(Self { e_stop_threshold_cm })
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let (get_nsm, get_hcsr04, get_opencviox2) = input;

        let nsm_msg = get_nsm.payload().map_or(Err(CuError::from(format!("none pload PropulsionAdapter"))), |msg| {Ok(msg)})?;
        let hcsr04_msg = get_hcsr04.payload().map_or(Err(CuError::from(format!("none payload hcsr04"))), |msg| {Ok(msg)})?;
        let opencviox2_msg = get_opencviox2.payload().map_or(Err(CuError::from(format!("none payload opencviox2"))), |msg| {Ok(msg)})?;

        let loop_state = nsm_msg.loop_state;

        let mut propulsion_payload
            = PropulsionPayload {
                left_enable: nsm_msg.left_enable,
                right_enable: nsm_msg.right_enable,
                left_speed: nsm_msg.openloop_left_speed,
                right_speed: nsm_msg.openloop_right_speed,
                left_direction: nsm_msg.left_direction,
                right_direction: nsm_msg.right_direction
            };

        let panner_payload = CameraPanningPayload { pos_cmd: nsm_msg.camera_position };
        let weighted_error = opencviox2_msg.heading_error;

        let mut is_e_stop_triggered = false;
        if hcsr04_msg.distance < self.e_stop_threshold_cm {
            is_e_stop_triggered = true;
        }

        let is_at_rest = match nsm_msg.work_or_rest_state {
            WorkOrRestState::AtRest => true,
            _ => false
        };

        let stop_condition = is_e_stop_triggered || is_at_rest;
        if stop_condition {
            propulsion_payload = PropulsionPayload {
                left_enable: false,
                right_enable: false,
                left_speed: 0.0,
                right_speed: 0.0,
                left_direction: WheelDirection::Stop,
                right_direction: WheelDirection::Stop,
            };
        }

        let output_payload = PropulsionAdapterOutputPayload {
            loop_state,
            propulsion_payload,
            panner_payload,
            weighted_error,
            is_e_stop_triggered,
            distance: hcsr04_msg.distance
        };
        output.set_payload(output_payload);
        Ok(())
    }
}
