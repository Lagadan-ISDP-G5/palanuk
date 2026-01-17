/// This task provides feedback to the base station. It runs the Zenoh publisher.

use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use propulsion_adapter::{LoopState, PropulsionAdapterOutputPayload};
use cu_propulsion::PropulsionPayload;
use cu_pid::PIDControlOutputPayload;

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct HeraldNewsPayload {
    e_stop_trig_fdbk: bool,
    loop_mode_fdbk: LoopState,
}

pub struct Herald {}

impl Freezable for Herald {}

impl CuSinkTask for Herald {
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload, PIDControlOutputPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self::default())
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        Ok(())
    }
}
