/// This task's sole purpose is to take out the PropulsionPayload out of the output tuple of
/// the Arbitrator task and to pass a PropulsionPayload by its own to the cu-propulsion sink task.

use cu29::prelude::*;
use cu_propulsion::PropulsionPayload;
use herald::HeraldNewsPayload;

pub struct PropulsionCleaver {}
impl Freezable for PropulsionCleaver {}

impl CuTask for PropulsionCleaver {
    type Input<'m> = input_msg!((PropulsionPayload, PropulsionPayload, HeraldNewsPayload));
    type Output<'m> = output_msg!(PropulsionPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        // TODO config should probably specify whether cleaver will cleave for left/right motor
        // config will then set some state value stored by PropulsionCleaver
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        if let Some(cleaved) = input.payload() {
            let (propulsion_cleaved_lmtr, propulsion_cleaved_rmtr, _) = cleaved;
            // TODO and then right here we'll read from Self to decide which left/right motor value to cleave for
            output.set_payload(*propulsion_cleaved);
        }
        Ok(())
    }
}
