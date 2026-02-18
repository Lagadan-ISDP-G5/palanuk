
use cu29::prelude::*;
use cu_irencoder::IrEncoderPayload;
use speed_ctrlrs::{LmtrSpeedErrPayload, RmtrSpeedErrPayload};
use cu_propulsion::PropulsionPayload;

pub struct SpeedErrAdapter {
    lmtr_speed_err: Option<f32>,
    rmtr_speed_err: Option<f32>,
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
            rmtr_speed_err: None
        })
    }

    fn process(&mut self, clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let (actual_speed, speed_setpoint) = (input.0.payload(), input.1.payload());
        // used in pid output calculations
        output.0.tov = Tov::Time(clock.now());
        output.1.tov = Tov::Time(clock.now());

        match (actual_speed, speed_setpoint) {
            (Some(actual_speed), Some(speed_setpoint)) => {
                let lmtr_actual_speed = actual_speed.lmtr_normalized_rpm;
                let rmtr_actual_speed = actual_speed.rmtr_normalized_rpm;

                match (lmtr_actual_speed, rmtr_actual_speed) {
                    (Some(lmtr_actual_speed), Some(rmtr_actual_speed)) => {
                        self.lmtr_speed_err = Some(speed_setpoint.left_speed - lmtr_actual_speed);
                        self.rmtr_speed_err = Some(speed_setpoint.right_speed - rmtr_actual_speed);
                    }
                    _ => ()
                }
            },
            _ => {
                match (self.lmtr_speed_err, self.rmtr_speed_err) {
                    (Some(lmtr_speed_err), Some(rmtr_speed_err)) => {
                        output.0.set_payload(LmtrSpeedErrPayload { error: lmtr_speed_err });
                        output.1.set_payload(RmtrSpeedErrPayload { error: rmtr_speed_err });
                    },
                    _ => ()
                }
            }
        }

        Ok(())
    }

}
