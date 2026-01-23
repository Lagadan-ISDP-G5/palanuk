/// This task provides feedback to the base station. It runs the Zenoh publisher.

use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use propulsion_adapter::{LoopState, PropulsionAdapterOutputPayload};
use cu_pid::PIDControlOutputPayload;

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct HeraldNewsPayload {
    pub e_stop_trig_fdbk: bool,
    pub loop_mode_fdbk: LoopState,
}

pub struct Herald {}

impl Freezable for Herald {}

impl CuSinkTask for Herald {
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload, PIDControlOutputPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>)
    -> CuResult<()>
    {
        let (prop_adap_output, pid_ctrl) = input;
        if let (Some(prop_adap_pload), Some(pid_ctrl_pload)) = (prop_adap_output.payload(), pid_ctrl.payload()) {
            // TODO send telemetry data over mpsc channel to a thread that runs the zenoh publisher
        }

        Ok(())
    }
}
