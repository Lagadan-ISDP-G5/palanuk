/// This task provides feedback to the base station. It runs the Zenoh publisher.

extern crate cu_bincode as bincode;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use propulsion_adapter::LoopState;
use cu_propulsion::PropulsionPayload;

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct AncPubPayload {
    pub e_stop_trig_fdbk: bool,
    pub loop_mode_fdbk: LoopState,
    pub distance: f64
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
pub struct ObstacleDetected(pub u8);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Distance(pub f64);

pub struct AncPub {}

impl Freezable for AncPub {}

impl CuTask for AncPub {
    type Input<'m> = input_msg!('m, AncPubPayload);
    // u8 - anc_obstacle
    // f64 - anc_distance

    type Output<'m> = output_msg!(ObstacleDetected, Distance);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        if let Some(anc_pub) = input.payload() {
            let obstacle_detected = anc_pub.e_stop_trig_fdbk;
            let distance_reading = Distance(anc_pub.distance);

            let obstacle_detected: ObstacleDetected = match obstacle_detected {
                false => ObstacleDetected(0),
                true => ObstacleDetected(1)
            };
            output.0.set_payload(obstacle_detected);
            output.1.set_payload(distance_reading);
        }

        Ok(())
    }
}
