
use cu29::prelude::*;
use cu_propulsion::PropulsionPayload;
use cu_pid::PIDControlOutputPayload;
use cu_irencoder::IrEncoderPayload;

pub const MAX_PID_CORRECTION: f32 = 0.25;

#[derive(Reflect)]
#[reflect(no_field_bounds, from_reflect = false)]
pub struct SpeedCorrectionSummer {
    #[reflect(ignore)]
    last_output: Option<PropulsionPayload>,
    k_ff_lmtr: f32,
    k_ff_rmtr: f32,
    max_pid_correction: f32,
    speed_correction_enabled: bool,
}

impl Default for SpeedCorrectionSummer {
    fn default() -> Self {
        Self {
            last_output: None,
            k_ff_lmtr: 1.0,
            k_ff_rmtr: 1.0,
            max_pid_correction: MAX_PID_CORRECTION,
            speed_correction_enabled: true,
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

                let speed_correction: String = kv
                    .get("speed_correction")
                    .expect("speed_correction not set in RON config. Valid values: \"enable\", \"disable\"")
                    .clone()
                    .into();
                inst.speed_correction_enabled = match speed_correction.as_str() {
                    "enable" => true,
                    "disable" => false,
                    _ => panic!("Invalid speed_correction value: \"{speed_correction}\". Valid values: \"enable\", \"disable\""),
                };

                if let Some(v) = kv.get("max_pid_correction") {
                    let f: f64 = v.clone().into();
                    inst.max_pid_correction = f as f32;
                }
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
        let _encoder = input.3.payload();

        if let Some(ff) = feedforward {
            let mut output_msg = ff.clone();

            if self.speed_correction_enabled {
                let lmtr_pid = lmtr_speed_ctrlr_outpload.map(|p| p.output).unwrap_or(0.0)
                    .clamp(-self.max_pid_correction, self.max_pid_correction);
                let rmtr_pid = rmtr_speed_ctrlr_outpload.map(|p| p.output).unwrap_or(0.0)
                    .clamp(-self.max_pid_correction, self.max_pid_correction);

                let lmtr_ff = ff.left_speed;
                let rmtr_ff = ff.right_speed;

                let lmtr_summed_speed = lmtr_pid + (self.k_ff_lmtr * lmtr_ff);
                let rmtr_summed_speed = rmtr_pid + (self.k_ff_rmtr * rmtr_ff);

                output_msg.left_speed = lmtr_summed_speed.clamp(0.0, 1.0);
                output_msg.right_speed = rmtr_summed_speed.clamp(0.0, 1.0);
            }

            self.last_output = Some(output_msg);
        }

        if let Some(msg) = self.last_output {
            output.set_payload(msg);
        }
        Ok(())
    }

}

