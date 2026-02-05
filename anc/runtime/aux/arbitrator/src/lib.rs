
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

pub const BASELINE_SPEED: f32 = 0.2;

/// r_wind_comp is applied to right motor. Depending on your winding resistance ratio difference
/// it can be more than or less than 1, but no greater than 2.
pub struct Arbitrator {
    e_stop_trig_fdbk: bool,
    loop_mode_fdbk: LoopState,
    target_speed: Option<f32>,
    /// Applied to right motor
    r_wind_comp: f32
}

impl Default for Arbitrator {
    fn default() -> Self {
        Self {
            e_stop_trig_fdbk: false,
            loop_mode_fdbk: LoopState::Closed,
            target_speed: None,
            r_wind_comp: 0.0
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

        let r_wind_comp: f64 = kv
            .get("r_wind_compensation")
            .expect("Motor winding resistance compensation factor not set in RON config")
            .clone()
            .into();

        let mut inst = Self::default();
        inst.r_wind_comp = r_wind_comp as f32;
        Ok(inst)
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        let (prop_adap, mtr_pid, _nsm) = *input;

        // PropulsionAdapterOutputPayload is required - can't do anything without it
        let Some(prop_adap_pload) = prop_adap.payload() else {
            return Ok(());
        };

        let loop_state = prop_adap_pload.loop_state;

        // FIXME
        self.target_speed = Some(prop_adap_pload.propulsion_payload.left_speed);

        let prop_payload = match loop_state {
            LoopState::Open => {
                // Open-loop: just pass through propulsion payload, no PID needed
                self.open_loop_handler(prop_adap_pload)?
            },
            LoopState::Closed => {
                // Closed-loop: requires PID output
                if let Some(mtr_pid_pload) = mtr_pid.payload() {
                    self.closed_loop_handler(mtr_pid_pload, prop_adap_pload)?
                } else {
                    // No PID output yet, use safe defaults (stopped)
                    PropulsionPayload::default()
                    // PropulsionPayload { left_enable: true, right_enable: true, left_speed: 0.15, right_speed: 0.15, left_direction: WheelDirection::Reverse, right_direction: WheelDirection::Reverse }
                }
            }
        };


        // NSM payload is for steering (TODO), not blocking on it

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

    fn closed_loop_handler(&self, pid_pload: &PIDControlOutputPayload, prop_adap_pload: &PropulsionAdapterOutputPayload) -> CuResult<PropulsionPayload> {

        if prop_adap_pload.is_e_stop_triggered {
            return Ok(PropulsionPayload::default());
        }

        let pid_output = pid_pload.output;


        // pid_output > 0 implies error >0, turn right: slow left, speed up right
        let left_speed = (self.target_speed.unwrap_or(BASELINE_SPEED) - pid_output).clamp(0.0, 0.9);
        // VERY IMPORTANT: apply compensation
        let right_speed = (self.target_speed.unwrap_or(BASELINE_SPEED)*self.r_wind_comp + pid_output).clamp(0.0, 0.9);

        Ok(PropulsionPayload {
            left_enable: true,
            right_enable: true,
            left_speed,
            right_speed,
            left_direction: WheelDirection::Forward,
            right_direction: WheelDirection::Forward,
        })
    }

    fn steering_handler(&mut self, steering_msg: NsmPayload) -> CuResult<PropulsionPayload> {
        todo!()
    }
}
