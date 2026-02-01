/// This task's sole purpose is to take out the PropulsionPayload out of the output tuple of
/// the Arbitrator task and to pass a PropulsionPayload by its own to the cu-propulsion sink task.

use cu29::prelude::*;
use cu_propulsion::PropulsionPayload;
use anc_pub::AncPubPayload;

// enum WhichMotor {
//     Lmtr,
//     Rmtr
// }

pub struct PropulsionCleaver {
    // which_motor: WhichMotor
}

impl Freezable for PropulsionCleaver {}

impl CuTask for PropulsionCleaver {
    type Input<'m> = input_msg!('m, PropulsionPayload);
    type Output<'m> = output_msg!(PropulsionPayload);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        // config should specify whether cleaver will cleave for left/right motor
        // config will then set some state value stored by PropulsionCleaver
        // let config = config.ok_or("No ComponentConfig specified for PropulsionCleaver in RON")?;

        // let which_motor: String = config
        //     .get::<String>("which_motor")
        //     .expect("which_motor for PropulsionCleaver not set in RON config.")
        //     .clone()
        //     .into();

        // let which_motor = match which_motor.as_str() {
        //     "lmtr" => WhichMotor::Lmtr,
        //     "rmtr" => WhichMotor::Rmtr,
        //     _ => {
        //         return Err(CuError::from(format!("which_motor for PropulsionCleaver can only be either 'lmtr' or 'rmtr'")))
        //     }
        // };

        // Ok(Self { which_motor})
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        if let Some(cleaved) = input.payload() {
            // and then right here we'll read from Self to decide which left/right motor value to cleave for
            // let cleaved = match self.which_motor {
            //     WhichMotor::Lmtr => propulsion_cleaved_lmtr,
            //     WhichMotor::Rmtr => propulsion_cleaved_rmtr,
            // };
            output.set_payload(*cleaved);
        }
        Ok(())
    }
}
