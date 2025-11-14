use dumb_sysfs_pwm::Pwm;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

// Not used here, the assignment is final but it should be passed in the RON instead of being hardcoded
const SG90_POS_CMD: u32 = 12;

/// this payload has no HW feedback
#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct CameraPanningPayload {
    pos_cmd: PositionCommand
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub enum PositionCommand {
    #[default]
    Front,
    Left,
    Right
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct CameraPanningPinAssignments {
    sg90_pos_cmd_pin: u32,
}

pub struct CameraPanningControllerInstances {
    sg90_pos_cmd: Pwm
}

pub struct CameraPanning {
    recvd_pos_cmd: PositionCommand,
    pin_controller_instances: CameraPanningControllerInstances,
    #[cfg(hardware)]
    pin_assignments: CameraPanningPinAssignments,
}

impl Freezable for CameraPanning {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.recvd_pos_cmd, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.recvd_pos_cmd = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuSinkTask for CameraPanning {
    type Input<'m> = input_msg!(CameraPanningPayload);

    fn new(config: Option<&ComponentConfig>) -> Result<Self, CuError>
    where Self: Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        let sg90_pos_cmd_pin_offset: u32 = kv
            .get("sg90_pos_cmd_pin")
            .expect("l298n_en_a for Propulsion not set in RON config")
            .clone()
            .into();

        let pin_assignments = CameraPanningPinAssignments {
            sg90_pos_cmd_pin: sg90_pos_cmd_pin_offset
        };

        #[cfg(hardware)]
        let sg90_pos_cmd_instance = Pwm::new(0, sg90_pos_cmd_pin_offset).unwrap();
        let pin_controller_instances = CameraPanningControllerInstances {
            sg90_pos_cmd: sg90_pos_cmd_instance
        };

        Ok(Self {
            recvd_pos_cmd: PositionCommand::default(),
            pin_controller_instances: pin_controller_instances,
            pin_assignments: pin_assignments,
        })
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>) -> Result<(), CuError> {

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
