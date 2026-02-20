
use cu29::prelude::*;
use cu_irencoder::IrEncoderPayload;
use speed_ctrlrs::{LmtrSpeedErrPayload, RmtrSpeedErrPayload};
use cu_propulsion::PropulsionPayload;

pub struct SpeedErrAdapter {
    lmtr_speed_err: Option<f32>,
    rmtr_speed_err: Option<f32>,
    lmtr_actual: f32,
    rmtr_actual: f32,
}

impl Freezable for SpeedErrAdapter {}

impl CuTask for SpeedErrAdapter {
    type Input<'m> = input_msg!('m, IrEncoderPayload, PropulsionPayload);
    type Output<'m> = output_msg!(LmtrSpeedErrPayload, RmtrSpeedErrPayload);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {
            lmtr_speed_err: None,
            rmtr_speed_err: None,
            lmtr_actual: 0.0,
            rmtr_actual: 0.0,
        })
    }

    fn process(&mut self, clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let actual_speed = input.0.payload();
        // used in pid output calculations
        output.0.tov = Tov::Time(clock.now());
        output.1.tov = Tov::Time(clock.now());

        if let Some(actual_speed) = actual_speed {
            if let Some(lmtr) = actual_speed.lmtr_normalized_rpm {
                self.lmtr_actual = lmtr.clamp(0.0, 0.9);
            }
            if let Some(rmtr) = actual_speed.rmtr_normalized_rpm {
                self.rmtr_actual = rmtr.clamp(0.0, 0.9);
            }
        }

        // lane PID controller will give a distribution ratio setpoint
        // in 'openloop' (blind, no vision input) the command left and right should be the same when the intention
        // is to go straihgt
        let cmd = input.1.payload();
        let (lmtr_target_ratio, rmtr_target_ratio) = if let Some(cmd) = cmd {
            let total_cmd = cmd.left_speed + cmd.right_speed;
            if total_cmd > 0.0 {
                (cmd.left_speed / total_cmd, cmd.right_speed / total_cmd)
            } else {
                (0.5, 0.5)
            }
        } else {
            (0.5, 0.5)
        };

        let total_actual = self.lmtr_actual + self.rmtr_actual;
        let lmtr_target = total_actual * lmtr_target_ratio;
        let rmtr_target = total_actual * rmtr_target_ratio;

        self.lmtr_speed_err = Some(self.lmtr_actual - lmtr_target);
        self.rmtr_speed_err = Some(self.rmtr_actual - rmtr_target);

        if let (Some(lmtr_speed_err), Some(rmtr_speed_err)) = (self.lmtr_speed_err, self.rmtr_speed_err) {
            output.0.set_payload(LmtrSpeedErrPayload { error: lmtr_speed_err });
            output.1.set_payload(RmtrSpeedErrPayload { error: rmtr_speed_err });
        }

        Ok(())
    }

}
