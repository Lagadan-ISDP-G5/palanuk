
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

pub const R_WIND_COMP_LMTR: f32 = 1.0; // 1.17
pub const R_WIND_COMP_RMTR: f32 = 1.0; // 0.85

pub const DEFAULT_BASELINE_SPEED: f32 = 0.7;
pub const DEFAULT_HEADING_ERROR_END_STEERING_MANEUVER_THRESHOLD: f32 = 0.18;
pub const DEFAULT_OUTER_WHEEL_STEERING_SPEED: f32 = 1.0;
pub const DEFAULT_INNER_WHEEL_STEERING_SPEED: f32 = 0.0;

pub const DEFAULT_ON_AXIS_ROTATION_DURATION_MILLISEC_90_DEG: u64 = 400;
pub const DEFAULT_STEERING_MIN_HOLD_MS: u64 = 300;
pub const DEFAULT_STEERING_DELAY_MS: u64 = 200;
pub const DEFAULT_STEERING_COOLDOWN_MS: u64 = 500;
pub const DEFAULT_STEERING_MAX_HOLD_MS: u64 = 2000;
pub const HEADING_CHANGE_MIN_DELTA: f32 = 0.05;

/// r_wind_comp values can be between 0 and 2 for either motor, but not both. If one is > 1 another must be <1.
#[derive(Reflect)]
#[reflect(no_field_bounds, from_reflect = false)]
pub struct Arbitrator {
    e_stop_trig_fdbk: bool,
    target_speed: Option<f32>,
    /// Applied to left motor
    r_wind_comp_lmtr: f32,
    /// Applied to right motor
    r_wind_comp_rmtr: f32,
    /// normalized corner y coord to trigger steering handler and override lanekeeping for the maneuver
    corner_y_coord_steering_trig: f32,
    baseline_speed: f32,
    heading_error_end_steering_maneuver_threshold: f32,
    outer_wheel_steering_speed: f32,
    inner_wheel_steering_speed: f32,
    steering_min_hold_ms: u64,
    steering_delay_ms: u64,
    steering_cooldown_ms: u64,
    steering_max_hold_ms: u64,
    #[reflect(ignore)]
    steerer_state: SteererState,
    #[reflect(ignore)]
    steering_direction: CornerDirection,
    #[reflect(ignore)]
    heading_error_at_steering_start: f32,
    #[reflect(ignore)]
    steering_triggered: CuInstant,
    #[reflect(ignore)]
    steering_started: CuInstant,
    #[reflect(ignore)]
    steering_completed: CuInstant,
    #[reflect(ignore)]
    on_axis_rotator: OnAxisRotator,
    last_pid_output: f32,
}

pub struct OnAxisRotator {
    current_cmd: RotateOnAxisCmd,
    last_cmd: RotateOnAxisCmd,
    rotator_state: RotateOnAxisState,
    instant_rotating_started: CuInstant,
    rotation_duration_ms: u64,
}

impl Default for OnAxisRotator {
    fn default() -> Self {
        Self {
            current_cmd: RotateOnAxisCmd::Free,
            last_cmd: RotateOnAxisCmd::Free,
            rotator_state: RotateOnAxisState::Init,
            instant_rotating_started: CuInstant::now(),
            rotation_duration_ms: DEFAULT_ON_AXIS_ROTATION_DURATION_MILLISEC_90_DEG,
        }
    }
}

/// routine duration determines angle of rotation
/// if we can tune ON_AXIS_ROTATION_DURATION_MILLISEC_90_DEG to do 90 deg we can trivially
/// extra/interpolate other angles
impl OnAxisRotator {
    fn update_current_cmd_from_wheel_dir(&mut self, left_wheel_dir: WheelDirection, right_wheel_dir: WheelDirection) {
        if self.rotator_state != RotateOnAxisState::Rotating {
            self.last_cmd = self.current_cmd;
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
        let dur = CuDuration::from_millis(self.rotation_duration_ms);
        let res = CuInstant::now().as_nanos().checked_sub(self.instant_rotating_started.as_nanos());
        let elapsed = CuDuration::from_nanos(res.unwrap_or(0u64));

        if elapsed >= dur && self.rotator_state == RotateOnAxisState::Rotating { is_cmd_done = true; }
        else { is_cmd_done = false; }

        if is_cmd_done {
            self.rotator_state = RotateOnAxisState::Done;
            (false, None)
        }
        else if self.rotator_state == RotateOnAxisState::Rotating {
            // this is where we do the motor command subroutine to rotate the rover on its axis
            (true, Some(self.current_cmd))
        }
        else {
            (false, None)
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

#[derive(Default, Debug, PartialEq, Eq)]
pub enum SteererState {
    WaitingToSteer,
    Steering,
    Done,
    Cooldown,
    #[default]
    NotSteering
}

impl Default for Arbitrator {
    fn default() -> Self {
        Self {
            e_stop_trig_fdbk: false,
            target_speed: None,
            r_wind_comp_lmtr: 0.0,
            r_wind_comp_rmtr: 0.0,
            corner_y_coord_steering_trig: 0.0,
            baseline_speed: DEFAULT_BASELINE_SPEED,
            heading_error_end_steering_maneuver_threshold: DEFAULT_HEADING_ERROR_END_STEERING_MANEUVER_THRESHOLD,
            outer_wheel_steering_speed: DEFAULT_OUTER_WHEEL_STEERING_SPEED,
            inner_wheel_steering_speed: DEFAULT_INNER_WHEEL_STEERING_SPEED,
            steering_min_hold_ms: DEFAULT_STEERING_MIN_HOLD_MS,
            steering_delay_ms: DEFAULT_STEERING_DELAY_MS,
            steering_cooldown_ms: DEFAULT_STEERING_COOLDOWN_MS,
            steering_max_hold_ms: DEFAULT_STEERING_MAX_HOLD_MS,
            steerer_state: SteererState::default(),
            steering_direction: CornerDirection::default(),
            heading_error_at_steering_start: 0.0,
            steering_triggered: CuInstant::now(),
            steering_started: CuInstant::now(),
            steering_completed: CuInstant::now(),
            on_axis_rotator: OnAxisRotator::default(),
            last_pid_output: 0.0,
        }
    }
}

impl Freezable for Arbitrator {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.e_stop_trig_fdbk, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.e_stop_trig_fdbk = Decode::decode(decoder)?;
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

        let baseline_speed: f32 = kv.get("baseline_speed")
            .map(|v| { let f: f64 = v.clone().into(); f as f32 })
            .unwrap_or(DEFAULT_BASELINE_SPEED);

        let heading_error_end_steering_maneuver_threshold: f32 = kv.get("heading_error_end_steering_threshold")
            .map(|v| { let f: f64 = v.clone().into(); f as f32 })
            .unwrap_or(DEFAULT_HEADING_ERROR_END_STEERING_MANEUVER_THRESHOLD);

        let outer_wheel_steering_speed: f32 = kv.get("outer_wheel_steering_speed")
            .map(|v| { let f: f64 = v.clone().into(); f as f32 })
            .unwrap_or(DEFAULT_OUTER_WHEEL_STEERING_SPEED);

        let inner_wheel_steering_speed: f32 = kv.get("inner_wheel_steering_speed")
            .map(|v| { let f: f64 = v.clone().into(); f as f32 })
            .unwrap_or(DEFAULT_INNER_WHEEL_STEERING_SPEED);

        let on_axis_rotation_duration_ms: u64 = kv.get("on_axis_rotation_duration_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(DEFAULT_ON_AXIS_ROTATION_DURATION_MILLISEC_90_DEG);

        let steering_min_hold_ms: u64 = kv.get("steering_min_hold_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(DEFAULT_STEERING_MIN_HOLD_MS);

        let steering_delay_ms: u64 = kv.get("steering_delay_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(DEFAULT_STEERING_DELAY_MS);

        let steering_cooldown_ms: u64 = kv.get("steering_cooldown_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(DEFAULT_STEERING_COOLDOWN_MS);

        let steering_max_hold_ms: u64 = kv.get("steering_max_hold_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(DEFAULT_STEERING_MAX_HOLD_MS);

        let mut inst = Self::default();
        inst.r_wind_comp_lmtr = r_wind_comp_lmtr as f32;
        inst.r_wind_comp_rmtr = r_wind_comp_rmtr as f32;
        inst.corner_y_coord_steering_trig = corner_y_coord_steering_trig as f32;
        inst.baseline_speed = baseline_speed;
        inst.heading_error_end_steering_maneuver_threshold = heading_error_end_steering_maneuver_threshold;
        inst.outer_wheel_steering_speed = outer_wheel_steering_speed;
        inst.inner_wheel_steering_speed = inner_wheel_steering_speed;
        inst.steering_min_hold_ms = steering_min_hold_ms;
        inst.steering_delay_ms = steering_delay_ms;
        inst.steering_cooldown_ms = steering_cooldown_ms;
        inst.steering_max_hold_ms = steering_max_hold_ms;
        inst.on_axis_rotator.rotation_duration_ms = on_axis_rotation_duration_ms;
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

        let mut closed_loop_prop_payload: PropulsionPayload = PropulsionPayload::default();
        // lanekeeping handler
        if let Some(mtr_pid_pload) = mtr_pid.payload() && self.steerer_state != SteererState::Steering {
            self.last_pid_output = mtr_pid_pload.output;
        }

        let loop_state = prop_adap_pload.loop_state;
        match loop_state {
            LoopState::Closed => {
                closed_loop_prop_payload = self.closed_loop_handler(self.last_pid_output, prop_adap_pload)?;

                // phase 1: nsm dependent trigger and cancel only
                if let Some(m) = nsm.payload() {
                    if m.corner_detected {
                        eprintln!("CORNER detected dir={:?} y={:.4} trig={:.4} state={:?}",
                            m.corner_direction, m.corner_coords.1,
                            self.corner_y_coord_steering_trig, self.steerer_state);
                    }

                    let corner_close_enough = m.corner_coords.1 >= self.corner_y_coord_steering_trig && m.corner_detected;

                    // only trigger from NotSteering (cooldown must expire first)
                    if corner_close_enough && self.steerer_state == SteererState::NotSteering {
                        self.steerer_state = SteererState::WaitingToSteer;
                        self.steering_triggered = CuInstant::now();
                        self.steering_direction = m.corner_direction;
                        eprintln!("STEERING: waiting {}ms before maneuver dir={:?}", self.steering_delay_ms, m.corner_direction);
                    }

                    // refine direction while waiting (vision may update)
                    if self.steerer_state == SteererState::WaitingToSteer && m.corner_detected {
                        self.steering_direction = m.corner_direction;
                    }

                    // cancel if corner disappears while still waiting
                    if self.steerer_state == SteererState::WaitingToSteer && !m.corner_detected {
                        self.steerer_state = SteererState::NotSteering;
                        eprintln!("STEERING: corner lost during delay, cancelling");
                    }
                }

                // phase 2: timer driven transitions (run every tick, not gated on nsm)
                if self.steerer_state == SteererState::WaitingToSteer {
                    let elapsed_ns = CuInstant::now().as_nanos()
                        .checked_sub(self.steering_triggered.as_nanos())
                        .unwrap_or(0);
                    if CuDuration::from_nanos(elapsed_ns) >= CuDuration::from_millis(self.steering_delay_ms) {
                        self.steerer_state = SteererState::Steering;
                        self.steering_started = CuInstant::now();
                        self.heading_error_at_steering_start = prop_adap_pload.weighted_error;
                        eprintln!("STEERING: delay elapsed, starting maneuver (initial heading_err={:.4})",
                            self.heading_error_at_steering_start);
                    }
                }

                if self.steerer_state == SteererState::Steering {
                    eprintln!("STEERING: heading_err={:.4} L={:.4} R={:.4}",
                        prop_adap_pload.weighted_error,
                        closed_loop_prop_payload.left_speed,
                        closed_loop_prop_payload.right_speed);
                    self.steering_handler(prop_adap_pload, &mut closed_loop_prop_payload);
                }

                if self.steerer_state == SteererState::Done {
                    self.steering_completed = CuInstant::now();
                    self.steerer_state = SteererState::Cooldown;
                    eprintln!("STEERING: maneuver done, entering {}ms cooldown", self.steering_cooldown_ms);
                }

                if self.steerer_state == SteererState::Cooldown {
                    let elapsed_ns = CuInstant::now().as_nanos()
                        .checked_sub(self.steering_completed.as_nanos())
                        .unwrap_or(0);
                    if CuDuration::from_nanos(elapsed_ns) >= CuDuration::from_millis(self.steering_cooldown_ms) {
                        self.steerer_state = SteererState::NotSteering;
                        eprintln!("STEERING: cooldown expired, ready for next corner");
                    }
                }
            }
            _ => ()
        }

        let prop_payload: PropulsionPayload = match loop_state {
            LoopState::Open => {
                // Open-loop: just pass through propulsion payload, no PID needed
                self.open_loop_handler(prop_adap_pload)?
            },
            LoopState::Closed => {
                closed_loop_prop_payload
            }
        };

        let anc_pub_pload = AncPubPayload {
            e_stop_trig_fdbk: prop_adap_pload.is_e_stop_triggered,
            loop_mode_fdbk: prop_adap_pload.loop_state,
            distance: prop_adap_pload.distance
        };

        output.0.set_payload(prop_payload);
        output.1.set_payload(anc_pub_pload);
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
            ret.right_speed = (ret.right_speed * self.r_wind_comp_rmtr).clamp(0.0, 1.0);
            ret.left_speed = (ret.left_speed * self.r_wind_comp_lmtr).clamp(0.0, 1.0);

            self.on_axis_rotator.update_current_cmd_from_wheel_dir(ret.left_direction, ret.right_direction);
            if self.on_axis_rotator.current_cmd != RotateOnAxisCmd::Free {
                let (is_rotating, _) = self.on_axis_rotator.should_rotate();
                if !is_rotating {
                    ret.left_direction = WheelDirection::Stop;
                    ret.right_direction = WheelDirection::Stop;
                }
            }

        }

        Ok(ret)
    }

    fn closed_loop_handler(&self, pid_output: f32, prop_adap_pload: &PropulsionAdapterOutputPayload) -> CuResult<PropulsionPayload> {
        if prop_adap_pload.is_e_stop_triggered {
            return Ok(PropulsionPayload::default());
        }

        // cu-pid: output = kp * (setpoint - input), so positive error gives negative output
        let base_speed = self.target_speed.unwrap_or(self.baseline_speed);
        // Anti-windup: clamp PID output so neither motor saturates at 0,
        // preventing the integrator from winding up against the clamp
        let pid_clamped = pid_output.clamp(-base_speed, base_speed);
        let left_speed = (base_speed + pid_clamped).clamp(0.0, 1.0);
        let right_speed = (base_speed - pid_clamped).clamp(0.0, 1.0);

        eprintln!("LANE PID={:.4} (clamped={:.4}) | base={:.4} | L={:.4} R={:.4}", pid_output, pid_clamped, base_speed, left_speed, right_speed);

        Ok(PropulsionPayload {
            left_enable: true,
            right_enable: true,
            left_speed,
            right_speed,
            left_direction: WheelDirection::Forward,
            right_direction: WheelDirection::Forward,
        })
    }

    /// usses latched steering_direction so it runs every tick without depending on nsm.
    /// exit conditions:
    ///   1. max hold exceeded to prevnet uturn
    ///   2. min hold expired && heading error is small && heading actually changed from start
    fn steering_handler(&mut self, prop_adap_pload: &PropulsionAdapterOutputPayload, res: &mut PropulsionPayload) {
        let heading_error = prop_adap_pload.weighted_error;

        let elapsed_ns = CuInstant::now().as_nanos()
            .checked_sub(self.steering_started.as_nanos())
            .unwrap_or(0);
        let elapsed = CuDuration::from_nanos(elapsed_ns);
        let hold_expired = elapsed >= CuDuration::from_millis(self.steering_min_hold_ms);
        let max_exceeded = elapsed >= CuDuration::from_millis(self.steering_max_hold_ms);

        let heading_small = heading_error.abs() < self.heading_error_end_steering_maneuver_threshold;
        let heading_changed = (heading_error - self.heading_error_at_steering_start).abs() > HEADING_CHANGE_MIN_DELTA;

        if max_exceeded {
            eprintln!("STEERING: max hold {}ms exceeded, forcing done", self.steering_max_hold_ms);
            self.steerer_state = SteererState::Done;
        } else if hold_expired && heading_small && heading_changed {
            self.steerer_state = SteererState::Done;
        } else {
            let (left_speed, right_speed) = match self.steering_direction {
                CornerDirection::Right => {
                    (self.inner_wheel_steering_speed * self.r_wind_comp_lmtr,
                     self.outer_wheel_steering_speed * self.r_wind_comp_rmtr)
                },
                CornerDirection::Left => {
                    (self.outer_wheel_steering_speed * self.r_wind_comp_lmtr,
                     self.inner_wheel_steering_speed * self.r_wind_comp_rmtr)
                }
            };
            res.left_speed = left_speed.clamp(0.0, 1.0);
            res.right_speed = right_speed.clamp(0.0, 1.0);
        }
    }

}
