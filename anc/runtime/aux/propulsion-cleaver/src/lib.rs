/// This task's sole purpose is to take out the PropulsionPayload out of the output tuple of
/// the Arbitrator task and to pass a PropulsionPayload by its own to the cu-propulsion sink task.

use cu29::prelude::*;
use cu_propulsion::PropulsionPayload;
use herald::HeraldNewsPayload;

pub struct PropulsionCleaver {}
impl Freezable for PropulsionCleaver {}

impl CuTask for PropulsionCleaver {
    type Input<'m> = input_msg!((PropulsionPayload, HeraldNewsPayload));
    type Output<'m> = output_msg!(PropulsionPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        if let Some(cleaved) = input.payload() {
            let (propulsion_cleaved, _) = cleaved;
            output.set_payload(*propulsion_cleaved);
        }
        Ok(())
    }
}
