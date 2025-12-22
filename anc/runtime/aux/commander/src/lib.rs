use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use cu_hcsr04::*;
use cu_zenoh_src::*;
use cu_cam_pan::*;
use cu_propulsion::*;

impl CuTask for Jogger {
    type Input<'m> = input_msg!('m, HcSr04Payload);
    type Output<'m> = output_msg!(PropulsionPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    // fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
    //     // use this method to init iox2 sub
    //     Ok(())
    // }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let hcsr04_msg = input;
        let mut dist: Option<f64> = None;

        match hcsr04_msg.payload() {
            Some(payload) => dist = Some(payload.distance),
            _ => {}
        }

        if dist < Some(10.0) {
            output.set_payload(PropulsionPayload {
                left_enable: false,
                right_enable: false,
                left_direction: WheelDirection::Stop,
                right_direction: WheelDirection::Stop,
                left_speed: 0.0,
                right_speed: 0.0
            });

            output.metadata.set_status(format!("Stopped. Obstacle detected."));
        }

        if dist > Some(10.0) || dist == None {
            output.set_payload(PropulsionPayload {
                left_enable: true,
                right_enable: true,
                left_direction: WheelDirection::Forward,
                right_direction: WheelDirection::Forward,
                left_speed: 0.25/MOTOR_COMPENSATION,
                right_speed: 0.25,
            });

            output.metadata.set_status(format!("Moving..."));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
