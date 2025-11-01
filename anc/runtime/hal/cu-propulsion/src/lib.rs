use dumb_sysfs_pwm::{Pwm, PwmChip};
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct PropulsionPayload {
    left_enable: bool,
    right_enable: bool,
    left_speed: f64,
    right_speed: f64,
    left_direction: WheelDirection,
    right_direction: WheelDirection
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub enum WheelDirection {
    #[default]
    Forward,
    Reverse
}

pub struct Wheel {
    enable: bool,
    direction: WheelDirection,
    speed: f64,
}

pub struct Propulsion {
    left_wheel: Wheel,
    right_wheel: Wheel
}

impl Freezable for Propulsion {}

impl CuSinkTask for Propulsion {
    type Input<'m> = input_msg!(PropulsionPayload)
    fn new(_config: Option<&ComponentConfig>) -> Result<Self, CuError>
        where Self: Sized
    {
        Ok(Self { left_wheel: (), right_wheel: () })
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>) -> Result<(), CuError> {
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
