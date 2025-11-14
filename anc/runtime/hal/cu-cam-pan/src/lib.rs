use dumb_sysfs_pwm::*;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};



/// `left_speed` and `right_speed` are the percentage duty cycles for the Pwm controllers of each wheel.
#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct PropulsionPayload {
    pos_cmd: PositionCommand
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub enum PositionCommand {
    #[default]
    Front,
    Left,
    Right
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
