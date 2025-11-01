use dumb_sysfs_pwm::*;

use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub enum WheelDirection {
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

impl CuSinkTask for Propulsion {

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
