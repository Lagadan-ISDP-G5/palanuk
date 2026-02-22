
use cu29::prelude::*;
use cu_propulsion::PropulsionPayload;
use cu_pid::PIDControlOutputPayload;
use cu_irencoder::IrEncoderPayload;

pub const R_WIND_COMP_LMTR: f32 = 1.1;
pub const R_WIND_COMP_RMTR: f32 = 1.0;
pub const MAX_PID_CORRECTION: f32 = 0.25;

pub const STALL_CMD_THRESHOLD: f32 = 0.15;
pub const STALL_SPEED_THRESHOLD: f32 = 0.05;
pub const STALL_BOOST: f32 = 0.3;

pub struct SpeedCorrectionSummer {
    last_output: Option<PropulsionPayload>,
    k_ff_lmtr: f32,
    k_ff_rmtr: f32
}

impl Default for SpeedCorrectionSummer {
    fn default() -> Self {
        Self {
            last_output: None,
            k_ff_lmtr: 1.0,
            k_ff_rmtr: 1.0
        }
    }
}

impl Freezable for SpeedCorrectionSummer {}

impl CuTask for SpeedCorrectionSummer {
    type Input<'m> = input_msg!('m, PIDControlOutputPayload, PIDControlOutputPayload, PropulsionPayload, IrEncoderPayload);
    type Output<'m> = output_msg!(PropulsionPayload);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        let mut inst = Self::default();

        match config {
            Some(cfg) => {
                let ComponentConfig(kv) = cfg;
                let _k_ff_lmtr: f64 = kv
                    .get("k_ff_lmtr")
                    .expect("cfg specified but k_ff_lmtr is None")
                    .clone()
                    .into();

                inst.k_ff_lmtr = _k_ff_lmtr as f32;

                let _k_ff_rmtr: f64 = kv
                    .get("k_ff_rmtr")
                    .expect("cfg specified but k_ff_rmtr is None")
                    .clone()
                    .into();

                inst.k_ff_rmtr = _k_ff_rmtr as f32;
            },
            None => ()
        }

        Ok(inst)
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let lmtr_speed_ctrlr_outpload = input.0.payload();
        let rmtr_speed_ctrlr_outpload = input.1.payload();
        let feedforward = input.2.payload();
        let encoder = input.3.payload();

        let (lmtr_stall_boost, rmtr_stall_boost) = self.motor_stall_handler(encoder);

        if let Some(ff) = feedforward {
            let lmtr_pid = lmtr_speed_ctrlr_outpload.map(|p| p.output).unwrap_or(0.0)
                .clamp(-MAX_PID_CORRECTION, MAX_PID_CORRECTION);
            let rmtr_pid = rmtr_speed_ctrlr_outpload.map(|p| p.output).unwrap_or(0.0)
                .clamp(-MAX_PID_CORRECTION, MAX_PID_CORRECTION);

            let lmtr_ff = ff.left_speed;
            let rmtr_ff = ff.right_speed;

            let lmtr_summed_speed = lmtr_pid + (self.k_ff_lmtr * lmtr_ff) + lmtr_stall_boost;
            let rmtr_summed_speed = rmtr_pid + (self.k_ff_rmtr * rmtr_ff) + rmtr_stall_boost;

            let mut output_msg = ff.clone();
            output_msg.left_speed = (R_WIND_COMP_LMTR * lmtr_summed_speed).clamp(0.0, 1.0);
            output_msg.right_speed = (R_WIND_COMP_RMTR * rmtr_summed_speed).clamp(0.0, 1.0);

            self.last_output = Some(output_msg);
        }

        if let Some(msg) = self.last_output {
            output.set_payload(msg);
        }
        Ok(())
    }

}

// TODO: test this
// if it doesnt work then we probably need a timer based subroutine that goes like this:
// stop motor, wait, then go full blast until cu-irencoder registers movement past a certain threshold
impl SpeedCorrectionSummer {
    fn motor_stall_handler(&self, encoder: Option<&IrEncoderPayload>) -> (f32, f32) {
        let last = match self.last_output {
            Some(ref lo) => lo,
            None => return (0.0, 0.0),
        };

        let enc = match encoder {
            Some(e) => e,
            None => return (0.0, 0.0),
        };

        let lmtr_actual = enc.lmtr_normalized_rpm.unwrap_or(0.0);
        let rmtr_actual = enc.rmtr_normalized_rpm.unwrap_or(0.0);

        let lmtr_boost = if last.left_speed >= STALL_CMD_THRESHOLD && lmtr_actual < STALL_SPEED_THRESHOLD {
            // eprintln!("STALL L: cmd={:.4} actual={:.4}", last.left_speed, lmtr_actual);
            STALL_BOOST
        } else {
            0.0
        };

        let rmtr_boost = if last.right_speed >= STALL_CMD_THRESHOLD && rmtr_actual < STALL_SPEED_THRESHOLD {
            // eprintln!("STALL R: cmd={:.4} actual={:.4}", last.right_speed, rmtr_actual);
            STALL_BOOST
        } else {
            0.0
        };

        (lmtr_boost, rmtr_boost)
    }
}
