
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
use itp_merger::ItpTopicsOutputPayload;
use core::default::*;

pub const R_WIND_COMP_LMTR: f32 = 1.0; // 1.17
pub const R_WIND_COMP_RMTR: f32 = 1.0; // 0.85

pub const BASELINE_SPEED: f32 = 0.7;
pub const HEADING_ERROR_END_STEERING_MANEUVER_THRESHOLD: f32 = 0.18;
pub const OUTER_WHEEL_STEERING_SPEED: f32 = 1.0;
pub const INNER_WHEEL_STEERING_SPEED: f32 = 0.0;

pub const ON_AXIS_ROTATION_DURATION_MILLISEC_90_DEG: u64 = 400;
pub const STEERING_MIN_HOLD_MS: u64 = 280;
pub const STEERING_DELAY_MS: u64 = 400;

pub const ROCK_RAMP_MS: u64 = 60;
pub const ROCK_FORWARD_HOLD_MS: u64 = 180;
pub const ROCK_REVERSE_MS: u64 = 150;
pub const ROCK_SPEED: f32 = 0.9;
pub const ROCK_WIGGLE_RATIO: f32 = 0.7;
pub const ROCK_MAX_CYCLES: u8 = 3;
pub const ROCK_FULL_SEND_MS: u64 = 400;

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
    #[reflect(ignore)]
    steerer_state: SteererState,
    #[reflect(ignore)]
    steering_triggered: CuInstant,
    #[reflect(ignore)]
    steering_started: CuInstant,
    #[reflect(ignore)]
    on_axis_rotator: OnAxisRotator,
    last_pid_output: f32,
    #[reflect(ignore)]
    rock_state: RockState,
    #[reflect(ignore)]
    rock_phase_started: CuInstant,
    /// normalized corner y coord below which rocking stops (corner visible and close enough = cleared bump)
    corner_y_coord_rock_stop: f32,
    rock_ramp_ms: u64,
    rock_forward_hold_ms: u64,
    rock_reverse_ms: u64,
    rock_speed: f32,
    rock_wiggle_ratio: f32,
    rock_max_cycles: u8,
    rock_full_send_ms: u64,
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
        let dur = CuDuration::from_millis(ON_AXIS_ROTATION_DURATION_MILLISEC_90_DEG);
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
    #[default]
    NotSteering
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum RockState {
    #[default]
    NotRocking,
    RampingForward { cycle: u8 },
    HoldForward { cycle: u8 },
    Reversing { cycle: u8 },
    FullSend,
    Done,
}

impl Default for Arbitrator {
    fn default() -> Self {
        Self {
            e_stop_trig_fdbk: false,
            target_speed: None,
            r_wind_comp_lmtr: 0.0,
            r_wind_comp_rmtr: 0.0,
            corner_y_coord_steering_trig: 0.0,
            steerer_state: SteererState::default(),
            steering_triggered: CuInstant::now(),
            steering_started: CuInstant::now(),
            on_axis_rotator: OnAxisRotator::default(),
            last_pid_output: 0.0,
            rock_state: RockState::default(),
            rock_phase_started: CuInstant::now(),
            corner_y_coord_rock_stop: 0.0,
            rock_ramp_ms: ROCK_RAMP_MS,
            rock_forward_hold_ms: ROCK_FORWARD_HOLD_MS,
            rock_reverse_ms: ROCK_REVERSE_MS,
            rock_speed: ROCK_SPEED,
            rock_wiggle_ratio: ROCK_WIGGLE_RATIO,
            rock_max_cycles: ROCK_MAX_CYCLES,
            rock_full_send_ms: ROCK_FULL_SEND_MS,
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
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload, PIDControlOutputPayload, NsmPayload, ItpTopicsOutputPayload);
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
            .expect("corner_y_coord_steering_trig not set in RON config")
            .clone()
            .into();

        let corner_y_coord_rock_stop: f64 = kv
            .get("corner_y_coord_rock_stop")
            .expect("corner_y_coord_rock_stop not set in RON config")
            .clone()
            .into();

        let rock_ramp_ms: u64 = kv.get("rock_ramp_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(ROCK_RAMP_MS);
        let rock_forward_hold_ms: u64 = kv.get("rock_forward_hold_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(ROCK_FORWARD_HOLD_MS);
        let rock_reverse_ms: u64 = kv.get("rock_reverse_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(ROCK_REVERSE_MS);
        let rock_speed: f32 = kv.get("rock_speed")
            .map(|v| { let f: f64 = v.clone().into(); f as f32 })
            .unwrap_or(ROCK_SPEED);
        let rock_wiggle_ratio: f32 = kv.get("rock_wiggle_ratio")
            .map(|v| { let f: f64 = v.clone().into(); f as f32 })
            .unwrap_or(ROCK_WIGGLE_RATIO);
        let rock_max_cycles: u8 = kv.get("rock_max_cycles")
            .map(|v| { let f: f64 = v.clone().into(); f as u8 })
            .unwrap_or(ROCK_MAX_CYCLES);
        let rock_full_send_ms: u64 = kv.get("rock_full_send_ms")
            .map(|v| { let f: f64 = v.clone().into(); f as u64 })
            .unwrap_or(ROCK_FULL_SEND_MS);

        let mut inst = Self::default();
        inst.r_wind_comp_lmtr = r_wind_comp_lmtr as f32;
        inst.r_wind_comp_rmtr = r_wind_comp_rmtr as f32;
        inst.corner_y_coord_steering_trig = corner_y_coord_steering_trig as f32;
        inst.corner_y_coord_rock_stop = corner_y_coord_rock_stop as f32;
        inst.rock_ramp_ms = rock_ramp_ms;
        inst.rock_forward_hold_ms = rock_forward_hold_ms;
        inst.rock_reverse_ms = rock_reverse_ms;
        inst.rock_speed = rock_speed;
        inst.rock_wiggle_ratio = rock_wiggle_ratio;
        inst.rock_max_cycles = rock_max_cycles;
        inst.rock_full_send_ms = rock_full_send_ms;
        Ok(inst)
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        let (prop_adap, mtr_pid, nsm, itp) = *input;

        // PropulsionAdapterOutputPayload is required - can't do anything without it
        let Some(prop_adap_pload) = prop_adap.payload() else {
            return Ok(());
        };

        self.target_speed = Some(prop_adap_pload.propulsion_payload.left_speed); // FIXME?

        let mut closed_loop_prop_payload: PropulsionPayload = PropulsionPayload::default();
        // lanekeeping handler
        if let Some(mtr_pid_pload) = mtr_pid.payload()
            && self.steerer_state != SteererState::Steering
            && self.rock_state == RockState::NotRocking
        {
            self.last_pid_output = mtr_pid_pload.output;
        }

        // rocking trigger from ITP (rising edge)
        if let Some(itp_pload) = itp.payload() {
            if itp_pload.bump_rock_cmd && self.rock_state == RockState::NotRocking {
                self.rock_state = RockState::RampingForward { cycle: 0 };
                self.rock_phase_started = CuInstant::now();
                eprintln!("ROCK: triggered, starting cycle 0");
            }
        }

        // rocking early exit: corner visible and low enough means we cleared the bump
        if self.rock_state != RockState::NotRocking && self.rock_state != RockState::Done {
            if let Some(m) = nsm.payload() {
                if m.corner_detected && m.corner_coords.1 >= self.corner_y_coord_rock_stop {
                    eprintln!("ROCK: corner visible y={:.4} >= stop_trig={:.4}, ending rock",
                        m.corner_coords.1, self.corner_y_coord_rock_stop);
                    self.rock_state = RockState::Done;
                }
            }
        }

        // rocking reset: once done and trigger goes away
        if self.rock_state == RockState::Done {
            let trigger_gone = itp.payload().map(|p| !p.bump_rock_cmd).unwrap_or(true);
            if trigger_gone {
                self.rock_state = RockState::NotRocking;
            }
        }

        let loop_state = prop_adap_pload.loop_state;
        match loop_state {
            LoopState::Closed => {
                closed_loop_prop_payload = self.closed_loop_handler(self.last_pid_output, prop_adap_pload)?;

                // rocking overrides everything except e-stop
                if self.rock_state != RockState::NotRocking && self.rock_state != RockState::Done {
                    self.rock_handler(&mut closed_loop_prop_payload);
                }
                // steering handler (only when not rocking)
                else if let Some(m) = nsm.payload() {
                    if m.corner_detected {
                        eprintln!("CORNER detected dir={:?} y={:.4} trig={:.4} state={:?}",
                            m.corner_direction, m.corner_coords.1,
                            self.corner_y_coord_steering_trig, self.steerer_state);
                    }
                    if m.corner_coords.1 >= self.corner_y_coord_steering_trig && m.corner_detected {
                        if self.steerer_state == SteererState::NotSteering {
                            self.steerer_state = SteererState::WaitingToSteer;
                            self.steering_triggered = CuInstant::now();
                            eprintln!("STEERING: waiting {}ms before maneuver", STEERING_DELAY_MS);
                        }
                    }

                    // IMPORTANT edge case!!!
                    if self.steerer_state == SteererState::Done && !m.corner_detected {
                        self.steerer_state = SteererState::NotSteering;
                    }

                    if self.steerer_state == SteererState::WaitingToSteer {
                        let elapsed_ns = CuInstant::now().as_nanos()
                            .checked_sub(self.steering_triggered.as_nanos())
                            .unwrap_or(0);
                        if CuDuration::from_nanos(elapsed_ns) >= CuDuration::from_millis(STEERING_DELAY_MS) {
                            self.steerer_state = SteererState::Steering;
                            self.steering_started = CuInstant::now();
                            eprintln!("STEERING: delay elapsed, starting maneuver");
                        }
                    }

                    if self.steerer_state == SteererState::Steering {
                        eprintln!("STEERING: heading_err={:.4} L={:.4} R={:.4}",
                            prop_adap_pload.weighted_error,
                            closed_loop_prop_payload.left_speed,
                            closed_loop_prop_payload.right_speed);
                        self.steering_handler(prop_adap_pload, *m, &mut closed_loop_prop_payload);
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
        let base_speed = self.target_speed.unwrap_or(BASELINE_SPEED);
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

    fn rock_elapsed_ms(&self) -> u64 {
        let elapsed_ns = CuInstant::now().as_nanos()
            .checked_sub(self.rock_phase_started.as_nanos())
            .unwrap_or(0);
        elapsed_ns / 1_000_000
    }

    fn rock_handler(&mut self, res: &mut PropulsionPayload) {
        let elapsed = self.rock_elapsed_ms();

        match self.rock_state {
            RockState::RampingForward { cycle } => {
                let ramp_frac = (elapsed as f32 / self.rock_ramp_ms as f32).min(1.0);
                let speed = self.rock_speed * ramp_frac;

                // wiggle: alternate which wheel leads each cycle
                if cycle % 2 == 0 {
                    res.left_speed = speed;
                    res.right_speed = speed * self.rock_wiggle_ratio;
                } else {
                    res.left_speed = speed * self.rock_wiggle_ratio;
                    res.right_speed = speed;
                }
                res.left_direction = WheelDirection::Forward;
                res.right_direction = WheelDirection::Forward;
                res.left_enable = true;
                res.right_enable = true;

                if elapsed >= self.rock_ramp_ms {
                    self.rock_state = RockState::HoldForward { cycle };
                    self.rock_phase_started = CuInstant::now();
                    eprintln!("ROCK: cycle {} ramp done, holding forward", cycle);
                }
            }
            RockState::HoldForward { cycle } => {
                // maintain full speed with wiggle
                if cycle % 2 == 0 {
                    res.left_speed = self.rock_speed;
                    res.right_speed = self.rock_speed * self.rock_wiggle_ratio;
                } else {
                    res.left_speed = self.rock_speed * self.rock_wiggle_ratio;
                    res.right_speed = self.rock_speed;
                }
                res.left_direction = WheelDirection::Forward;
                res.right_direction = WheelDirection::Forward;
                res.left_enable = true;
                res.right_enable = true;

                if elapsed >= self.rock_forward_hold_ms {
                    self.rock_state = RockState::Reversing { cycle };
                    self.rock_phase_started = CuInstant::now();
                    eprintln!("ROCK: cycle {} hold done, reversing", cycle);
                }
            }
            RockState::Reversing { cycle } => {
                // symmetric reverse, no wiggle
                res.left_speed = self.rock_speed;
                res.right_speed = self.rock_speed;
                res.left_direction = WheelDirection::Reverse;
                res.right_direction = WheelDirection::Reverse;
                res.left_enable = true;
                res.right_enable = true;

                if elapsed >= self.rock_reverse_ms {
                    if cycle + 1 >= self.rock_max_cycles {
                        self.rock_state = RockState::FullSend;
                        self.rock_phase_started = CuInstant::now();
                        eprintln!("ROCK: max cycles reached, full send");
                    } else {
                        self.rock_state = RockState::RampingForward { cycle: cycle + 1 };
                        self.rock_phase_started = CuInstant::now();
                        eprintln!("ROCK: starting cycle {}", cycle + 1);
                    }
                }
            }
            RockState::FullSend => {
                // final ramped forward push, symmetric
                let ramp_frac = (elapsed as f32 / self.rock_ramp_ms as f32).min(1.0);
                res.left_speed = self.rock_speed * ramp_frac;
                res.right_speed = self.rock_speed * ramp_frac;
                res.left_direction = WheelDirection::Forward;
                res.right_direction = WheelDirection::Forward;
                res.left_enable = true;
                res.right_enable = true;

                if elapsed >= self.rock_full_send_ms {
                    self.rock_state = RockState::Done;
                    eprintln!("ROCK: full send done, routine complete");
                }
            }
            RockState::NotRocking | RockState::Done => {}
        }
    }

    /// called when corner y coord is low enough
    fn steering_handler(&mut self, prop_adap_pload: &PropulsionAdapterOutputPayload, steering_msg: NsmPayload, res: &mut PropulsionPayload) {
        // again, two possible sources of heading error
        // either offset calculated from the normalized corner x coord
        // or offset calculated from the vertical line that fits the center lane
        // this is decided in the propulsion-adapter task

        let heading_error = prop_adap_pload.weighted_error;
        let left_speed;
        let right_speed;

        let elapsed_ns = CuInstant::now().as_nanos()
            .checked_sub(self.steering_started.as_nanos())
            .unwrap_or(0);
        let hold_expired = CuDuration::from_nanos(elapsed_ns)
            >= CuDuration::from_millis(STEERING_MIN_HOLD_MS);

        if hold_expired && heading_error.abs() < HEADING_ERROR_END_STEERING_MANEUVER_THRESHOLD {
            self.steerer_state = SteererState::Done;
        }
        else {
            match steering_msg.corner_direction {
                CornerDirection::Right => {
                    left_speed = INNER_WHEEL_STEERING_SPEED * self.r_wind_comp_lmtr;
                    right_speed = OUTER_WHEEL_STEERING_SPEED * self.r_wind_comp_rmtr;
                },
                CornerDirection::Left => {
                    left_speed = OUTER_WHEEL_STEERING_SPEED * self.r_wind_comp_lmtr;
                    right_speed = INNER_WHEEL_STEERING_SPEED * self.r_wind_comp_rmtr;
                }
            }
            res.left_speed = left_speed.clamp(0.0, 1.0);
            res.right_speed = right_speed.clamp(0.0, 1.0);
        }
    }

}
