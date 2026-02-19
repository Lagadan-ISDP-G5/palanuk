
use cu29::prelude::*;
use cu_propulsion::PropulsionPayload;
use cu_pid::PIDControlOutputPayload;

pub const R_WIND_COMP_LMTR: f32 = 1.0;
pub const R_WIND_COMP_RMTR: f32 = 1.0;

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
    type Input<'m> = input_msg!('m, PIDControlOutputPayload, PIDControlOutputPayload, PropulsionPayload);
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

        let lmtr_summed_speed;
        let rmtr_summed_speed;

        match (lmtr_speed_ctrlr_outpload, rmtr_speed_ctrlr_outpload, feedforward) {
            (
                Some(lmtr_speed_ctrlr),
                Some(rmtr_speed_ctrlr),
                Some(ff)
            ) => {
                let lmtr_ff = ff.left_speed;
                let rmtr_ff = ff.right_speed;

                lmtr_summed_speed = lmtr_speed_ctrlr.output + (self.k_ff_lmtr * lmtr_ff);
                rmtr_summed_speed = rmtr_speed_ctrlr.output + (self.k_ff_rmtr * rmtr_ff);

                let mut output_msg = ff.clone();
                output_msg.left_speed = R_WIND_COMP_LMTR * lmtr_summed_speed.clamp(self.k_ff_lmtr * lmtr_ff, 1.0);
                output_msg.right_speed = R_WIND_COMP_RMTR * rmtr_summed_speed.clamp(self.k_ff_rmtr * rmtr_ff, 1.0);

                self.last_output = Some(output_msg);
            },
            _ => return Err(CuError::from(format!("last_output unset!")))
        }

        match self.last_output {
            Some(msg) => {
                output.set_payload(msg);
            },
            None => return Err(CuError::from(format!("no cmd sent to mtrs!")))
        }
        Ok(())
    }

}
