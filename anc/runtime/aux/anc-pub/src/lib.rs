/// This task provides feedback to the base station. It runs the Zenoh publisher.

use cu29::prelude::*;
use cu_bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use propulsion_adapter::{LoopState, PropulsionAdapterOutputPayload};
use cu_pid::PIDControlOutputPayload;

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct AncPubPayload {
    pub e_stop_trig_fdbk: bool,
    pub loop_mode_fdbk: LoopState,
    pub distance: f64
}

pub struct AncPub {}

impl Freezable for AncPub {}

impl CuTask for AncPub {
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload, PIDControlOutputPayload);
    // u8 - anc_obstacle
    // f64 - anc_distance

    type Output<'m> = output_msg!((u8, f64));
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        let (prop_adap_output, _pid_ctrl) = input;
        // if let (Some(prop_adap_pload), Some(pid_ctrl_pload)) = (prop_adap_output.payload(), pid_ctrl.payload()) {
        if let Some(prop_adap_pload)= prop_adap_output.payload() {
            let obstacle_detected = prop_adap_pload.is_e_stop_triggered;
            let distance_reading = prop_adap_pload.distance;

            let obstacle_detected: u8 = match obstacle_detected {
                false => 0,
                true => 1
            };

            output.set_payload((obstacle_detected, distance_reading));
        }

        Ok(())
    }
}
