
/// This task consolidates the arbitration of permissives/interlocks from different inputs
/// This is where it decides that an e-stop condition is correct, and also ultimately decides
/// if the loop mode can change. This task is stateful; it has feedback values for e-stop trigger
/// and loop mode.

extern crate cu_bincode as bincode;

use cu_pid::PIDControlOutputPayload;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use propulsion_adapter::{LoopState, PropulsionAdapterOutputPayload};
use cu_propulsion::{PropulsionPayload, WheelDirection};
use anc_pub::AncPubPayload;
use opencv_splitter::NsmPayload;
use opencv_iox2::{CornerDirection};
use core::default::*;

pub const R_WIND_COMP_LMTR: f32 = 1.17;
pub const R_WIND_COMP_RMTR: f32 = 0.85;

pub const BASELINE_SPEED: f32 = 0.2;
pub const HEADING_ERROR_END_STEERING_MANEUVER_THRESHOLD: f32 = 0.1;
pub const OUTER_WHEEL_STEERING_SPEED: f32 = 0.85;
pub const INNER_WHEEL_STEERING_SPEED: f32 = 0.5;

pub const ON_AXIS_ROTATION_DURATION_MILLISEC: u64 = 5250;

/// r_wind_comp values can be between 0 and 2 for either motor, but not both. If one is > 1 another must be <1.
pub struct Arbitrator {
    e_stop_trig_fdbk: bool,
    loop_mode_fdbk: LoopState,
    target_speed: Option<f32>,
    /// Applied to left motor
    r_wind_comp_lmtr: f32,
    /// Applied to right motor
    r_wind_comp_rmtr: f32,
    /// normalized corner y coord to trigger steering handler and override lanekeeping for the maneuver
    corner_y_coord_steering_trig: f32,
    steerer_state: SteererState,
    on_axis_rotator: OnAxisRotator,
}

pub struct OnAxisRotator {
    current_cmd: RotateOnAxisCmd,
    last_cmd: RotateOnAxisCmd,
    rotator_state: RotateOnAxisState,
    instant_rotating_started: CuInstant,
}

impl Default for OnAxisRotator {
    fn default() -> Self {
        Self {
            current_cmd: RotateOnAxisCmd::Free,
            last_cmd: RotateOnAxisCmd::Free,
            rotator_state: RotateOnAxisState::Init,
            instant_rotating_started: CuInstant::now()
        }
    }
}

impl OnAxisRotator {
    fn update_current_cmd_from_wheel_dir(&mut self, left_wheel_dir: WheelDirection, right_wheel_dir: WheelDirection) {
        if self.rotator_state != RotateOnAxisState::Rotating {
            match (left_wheel_dir, right_wheel_dir) {
                (WheelDirection::Forward, WheelDirection::Reverse) => {
                    self.current_cmd = RotateOnAxisCmd::RotateRight;
                },
                (WheelDirection::Reverse, WheelDirection::Forward) => {
                    self.current_cmd = RotateOnAxisCmd::RotateLeft;
                }
                _ => self.current_cmd = RotateOnAxisCmd::Free
            }
        }
    }

    /// returns a tuple:
    /// (false, None) -> dont do anything
    /// (true, Some(RotateOnAxisCmd)) -> do according to the RotateOnAxisCmd
    fn should_rotate(&mut self) -> (bool, Option<RotateOnAxisCmd>) {
        // only respond to rising edges
        let is_cmd_valid = match (self.last_cmd, self.current_cmd) {
            (RotateOnAxisCmd::Free, RotateOnAxisCmd::RotateLeft) => { true },
            (RotateOnAxisCmd::Free, RotateOnAxisCmd::RotateRight) => { true },
            (RotateOnAxisCmd::RotateLeft, RotateOnAxisCmd::RotateRight) => { true },
            (RotateOnAxisCmd::RotateRight, RotateOnAxisCmd::RotateLeft) => { true },
            _ => false
        };

        if is_cmd_valid && self.rotator_state != RotateOnAxisState::Rotating {
            self.instant_rotating_started = CuInstant::now();
            self.rotator_state = RotateOnAxisState::Rotating;
        }

        let is_cmd_done;
        let dur = CuDuration::from_millis(ON_AXIS_ROTATION_DURATION_MILLISEC);
        let res = CuInstant::now().as_nanos().checked_sub(self.instant_rotating_started.as_nanos());
        let elapsed = CuDuration::from_nanos(res.unwrap_or(0u64));

        if elapsed >= dur && is_cmd_valid { is_cmd_done = true; }
        else { is_cmd_done =  false; }

        if is_cmd_done {
            self.rotator_state = RotateOnAxisState::Done;
            (false, None)
        }
        else {
            // this is where we do the motor command subroutine to rotate the rover on its axis
            (true, Some(self.current_cmd))
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum RotateOnAxisCmd {
    Free,
    RotateLeft,
    RotateRight
}

#[derive(Clone, Copy, PartialEq)]
pub enum RotateOnAxisState {
    Init,
    Rotating,
    Done
}

#[derive(Default, PartialEq, Eq)]
pub enum SteererState {
    Steering,
    Done,
    #[default]
    NotSteering
}

impl Default for Arbitrator {
    fn default() -> Self {
        Self {
            e_stop_trig_fdbk: false,
            loop_mode_fdbk: LoopState::Closed,
            target_speed: None,
            r_wind_comp_lmtr: 0.0,
            r_wind_comp_rmtr: 0.0,
            corner_y_coord_steering_trig: 0.0,
            steerer_state: SteererState::default(),
            on_axis_rotator: OnAxisRotator::default()
        }
    }
}

impl Freezable for Arbitrator {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.e_stop_trig_fdbk, encoder)?;
        Encode::encode(&self.loop_mode_fdbk, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.e_stop_trig_fdbk = Decode::decode(decoder)?;
        self.loop_mode_fdbk = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuTask for Arbitrator {
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload, PIDControlOutputPayload, NsmPayload);
    type Output<'m> = output_msg!(PropulsionPayload, AncPubPayload);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        // let r_wind_comp_lmtr: f64 = kv
        //     .get("r_wind_comp_lmtr")
        //     .expect("Left motor winding resistance compensation factor not set in RON config")
        //     .clone()
        //     .into();
        let r_wind_comp_lmtr = R_WIND_COMP_LMTR;

        // let r_wind_comp_rmtr: f64 = kv
        //     .get("r_wind_comp_rmtr")
        //     .expect("Right motor winding resistance compensation factor not set in RON config")
        //     .clone()
        //     .into();
        let r_wind_comp_rmtr = R_WIND_COMP_RMTR;

        let corner_y_coord_steering_trig: f64 = kv
            .get("corner_y_coord_steering_trig")
            .expect("Normalized corner y coord trigger for steering not set in RON config")
            .clone()
            .into();

        let mut inst = Self::default();
        inst.r_wind_comp_lmtr = r_wind_comp_lmtr as f32;
        inst.r_wind_comp_rmtr = r_wind_comp_rmtr as f32;
        inst.corner_y_coord_steering_trig = corner_y_coord_steering_trig as f32;
        Ok(inst)
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        let (prop_adap, mtr_pid, nsm) = *input;

        // PropulsionAdapterOutputPayload is required - can't do anything without it
        let Some(prop_adap_pload) = prop_adap.payload() else {
            return Ok(());
        };

        self.target_speed = Some(prop_adap_pload.propulsion_payload.left_speed); // FIXME?

        // No PID output yet, use safe defaults (stopped)
        let mut closed_loop_prop_payload: PropulsionPayload = PropulsionPayload::default();
        // lanekeeping handler
        if let Some(mtr_pid_pload) = mtr_pid.payload() && self.steerer_state != SteererState::Steering {
            closed_loop_prop_payload = self.closed_loop_handler(mtr_pid_pload, prop_adap_pload)?;
        }
        // steering handler
        if let Some(m) = nsm.payload() {
            if m.corner_coords.1 <= self.corner_y_coord_steering_trig {
                self.steerer_state = SteererState::Steering; // sticky condition, will be mutated by steering handler
            }

            if self.steerer_state == SteererState::Steering {
                closed_loop_prop_payload = self.steering_handler(prop_adap_pload, *m)?;
            }
        }

        let loop_state = prop_adap_pload.loop_state;
        let prop_payload: PropulsionPayload = match loop_state {
            LoopState::Open => {
                // Open-loop: just pass through propulsion payload, no PID needed
                self.open_loop_handler(prop_adap_pload)?
            },
            LoopState::Closed => {
                closed_loop_prop_payload
            }
        };

        let herald_pload = AncPubPayload {
            e_stop_trig_fdbk: prop_adap_pload.is_e_stop_triggered,
            loop_mode_fdbk: prop_adap_pload.loop_state,
            distance: prop_adap_pload.distance
        };

        output.0.set_payload(prop_payload);
        output.1.set_payload(herald_pload);
        Ok(())
    }
}

impl Arbitrator {
    fn open_loop_handler(&mut self, prop_adap_pload: &PropulsionAdapterOutputPayload) -> CuResult<PropulsionPayload> {
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
            // VERY IMPORTANT: apply compensation
            ret.right_speed = ret.right_speed * self.r_wind_comp_rmtr;
            ret.left_speed = ret.left_speed * self.r_wind_comp_lmtr;

            self.on_axis_rotator.update_current_cmd_from_wheel_dir(ret.left_direction, ret.right_direction);
            if self.on_axis_rotator.current_cmd != RotateOnAxisCmd::Free {
                if let (should_rotate, Some(_cmd)) = self.on_axis_rotator.should_rotate() {
                    if !should_rotate {
                        ret.left_direction = WheelDirection::Stop;
                        ret.right_direction = WheelDirection::Stop;
                    }
                }
            }

        }

        Ok(ret)
    }

    fn closed_loop_handler(&self, pid_pload: &PIDControlOutputPayload, prop_adap_pload: &PropulsionAdapterOutputPayload) -> CuResult<PropulsionPayload> {
        if prop_adap_pload.is_e_stop_triggered {
            return Ok(PropulsionPayload::default());
        }

        let pid_output = pid_pload.output;
        // pid_output > 0 implies error >0, turn right: slow left, speed up right
        // // VERY IMPORTANT: apply compensation
        let left_speed = (
                (self.target_speed.unwrap_or(BASELINE_SPEED) - pid_output) * self.r_wind_comp_lmtr
            ).clamp(0.0, 0.9);

        let right_speed = (
                (self.target_speed.unwrap_or(BASELINE_SPEED) + pid_output) * self.r_wind_comp_rmtr
            ).clamp(0.0, 0.9);

        Ok(PropulsionPayload {
            left_enable: true,
            right_enable: true,
            left_speed,
            right_speed,
            left_direction: WheelDirection::Forward,
            right_direction: WheelDirection::Forward,
        })
    }

    /// called when corner y coord is low enough
    fn steering_handler(&mut self, prop_adap_pload: &PropulsionAdapterOutputPayload, steering_msg: NsmPayload) -> CuResult<PropulsionPayload> {
        // again, two possible sources of heading error
        // either offset calculated from the normalized corner x coord
        // or offset calculated from the vertical line that fits the center lane
        // this is decided in the propulsion-adapter task

        let heading_error = prop_adap_pload.weighted_error;
        let mut left_speed;
        let mut right_speed;

        match steering_msg.corner_direction {
            CornerDirection::Right => {
                right_speed = INNER_WHEEL_STEERING_SPEED * self.r_wind_comp_rmtr;
                left_speed = OUTER_WHEEL_STEERING_SPEED * self.r_wind_comp_lmtr;
            },
            CornerDirection::Left => {
                right_speed = OUTER_WHEEL_STEERING_SPEED * self.r_wind_comp_rmtr;
                left_speed = INNER_WHEEL_STEERING_SPEED * self.r_wind_comp_lmtr;
            } // unimplemented
        }

        if heading_error.abs() < HEADING_ERROR_END_STEERING_MANEUVER_THRESHOLD {
            right_speed = 0.0;
            left_speed = 0.0;
            self.steerer_state = SteererState::Done;
        }

        Ok(PropulsionPayload {
            left_enable: true,
            right_enable: true,
            left_speed,
            right_speed,
            left_direction: WheelDirection::Forward,
            right_direction: WheelDirection::Forward,
        })
    }

}
