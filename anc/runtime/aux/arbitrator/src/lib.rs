
/// This task consolidates the arbitration of permissives/interlocks from different inputs
/// This is where it decides that an e-stop condition is correct, and also ultimately decides
/// if the loop mode can change. This task is stateful; it has feedback values for e-stop trigger
/// and loop mode.

use cu29::prelude::*;
use bincode::{Decode, Encode};
// use serde::{Deserialize, Serialize};
use propulsion_adapter::{LoopState, PropulsionAdapterOutputPayload};
use cu_propulsion::{PropulsionPayload, WheelDirection};
use cu_pid::PIDControlOutputPayload;
use anc_pub::AncPubPayload;

pub const BASELINE_SPEED: f32 = 0.10;

pub struct Arbitrator {
    e_stop_trig_fdbk: bool,
    loop_mode_fdbk: LoopState,
    current_left_speed: f32,
    current_right_speed: f32
}

impl Default for Arbitrator {
    fn default() -> Self {
        Self {
            e_stop_trig_fdbk: false,
            loop_mode_fdbk: LoopState::Closed,
            current_left_speed: BASELINE_SPEED,
            current_right_speed: BASELINE_SPEED
        }
    }
}

impl Freezable for Arbitrator {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.e_stop_trig_fdbk, encoder)?;
        Encode::encode(&self.loop_mode_fdbk, encoder)?;
        Encode::encode(&self.current_left_speed, encoder)?;
        Encode::encode(&self.current_right_speed, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.e_stop_trig_fdbk = Decode::decode(decoder)?;
        self.loop_mode_fdbk = Decode::decode(decoder)?;
        self.current_left_speed = Decode::decode(decoder)?;
        self.current_right_speed = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuTask for Arbitrator {
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload, PIDControlOutputPayload);
    type Output<'m> = output_msg!((PropulsionPayload, AncPubPayload));

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self::default())
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        let (prop_adap, mtr_pid) = *input;
        if let (Some(prop_adap_pload), Some(mtr_pid_pload)) = (prop_adap.payload(), mtr_pid.payload()) {

            let prop_payload;
            let loop_state = prop_adap_pload.loop_state;
            match loop_state {
                LoopState::Closed => {
                    prop_payload = self.closed_loop_handler(mtr_pid_pload, prop_adap_pload)?;
                },
                LoopState::Open => {
                    prop_payload = self.open_loop_handler(prop_adap_pload)?;
                }
            }

            let herald_pload = AncPubPayload {
                e_stop_trig_fdbk: prop_adap_pload.is_e_stop_triggered,
                loop_mode_fdbk: prop_adap_pload.loop_state,
                distance: prop_adap_pload.distance
            };

            let arbitrator_output_payload = (prop_payload, herald_pload);
            output.set_payload(arbitrator_output_payload);
        }
        Ok(())
    }
}

impl Arbitrator {
    fn open_loop_handler(&self, prop_adap_pload: &PropulsionAdapterOutputPayload) -> CuResult<PropulsionPayload> {

        // initialize to safe conditions
        let left_enable: bool = false;
        let right_enable: bool = false;
        let left_speed: f32 = 0.0;
        let right_speed: f32 = 0.0;
        let left_direction: WheelDirection = WheelDirection::Stop;
        let right_direction: WheelDirection = WheelDirection::Stop;
        let mut ret
            = PropulsionPayload {
                left_enable,
                right_enable,
                left_speed,
                right_speed,
                left_direction,
                right_direction
            };

        if prop_adap_pload.is_e_stop_triggered {
            return Ok(ret)
        }

        else {
            ret = prop_adap_pload.propulsion_payload;
        }

        Ok(ret)
    }

    fn closed_loop_handler(&mut self, pid_pload: &PIDControlOutputPayload, prop_adap_pload: &PropulsionAdapterOutputPayload) -> CuResult<PropulsionPayload> {

        let pid_output = pid_pload.output;

        // initialize to safe conditions
        let mut left_enable: bool = false;
        let mut right_enable: bool = false;
        let left_speed: f32 = 0.0;
        let right_speed: f32 = 0.0;
        let mut left_direction: WheelDirection = WheelDirection::Stop;
        let mut right_direction: WheelDirection = WheelDirection::Stop;
        let mut ret
            = PropulsionPayload {
                left_enable,
                right_enable,
                left_speed,
                right_speed,
                left_direction,
                right_direction
            };

        let closedloop_left_speed = &mut self.current_left_speed;
        let closedloop_right_speed = &mut self.current_right_speed;

        if prop_adap_pload.is_e_stop_triggered {
            return Ok(ret)
        }

        else {
            left_enable = true;
            right_enable = true;
            left_direction = WheelDirection::Forward;
            right_direction = WheelDirection::Forward;

            if *closedloop_left_speed == 0.0 { *closedloop_left_speed = BASELINE_SPEED; }
            if *closedloop_right_speed == 0.0 { *closedloop_right_speed = BASELINE_SPEED; }

            let signum = pid_output.signum();
            if !signum.is_nan() {
                if signum == 1.0 {
                    *closedloop_left_speed = *closedloop_left_speed - pid_output;
                    *closedloop_right_speed = *closedloop_right_speed + pid_output;
                }
                if signum == -1.0 {
                    *closedloop_left_speed = *closedloop_left_speed + pid_output;
                    *closedloop_right_speed = *closedloop_right_speed - pid_output;
                }

                if *closedloop_left_speed > 1.0 { return Err(CuError::from(format!("left speed oversaturated"))) }
                if *closedloop_left_speed < 0.0 { return Err(CuError::from(format!("left speed undersaturated"))) }

                if *closedloop_right_speed > 1.0 { return Err(CuError::from(format!("right speed oversaturated"))) }
                if *closedloop_right_speed < 0.0 { return Err(CuError::from(format!("right speed undersaturated"))) }
            }
            else {
                return Err(CuError::from(format!("NaN encountered")))
            }

            ret = PropulsionPayload {
                left_enable,
                right_enable,
                left_speed: *closedloop_left_speed,
                right_speed: *closedloop_right_speed,
                left_direction,
                right_direction
            };
        }
        Ok(ret)
    }
}
