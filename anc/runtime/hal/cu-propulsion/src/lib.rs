use dumb_sysfs_pwm::{Pwm, PwmChip};
use gpio_cdev::*;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

// Not used here, the assignment is final but it should be passed in the RON instead of being hardcoded
const LMTR_ENABLE_PIN: u32 = 18;
const LMTR_IN_1: u32 = 23;
const LMTR_IN_2: u32 = 24;

const RMTR_ENABLE_PIN: u32 = 13;
const RMTR_IN_3: u32 = 26;
const RMTR_IN_4: u32 = 19;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct WheelState {
    enable: bool,
    direction: WheelDirection,
    speed: f64,
}

impl WheelState {
    fn default() -> Self {
        Self { enable: false, direction: WheelDirection::Forward, speed: 0.0 }
    }
}

pub struct PropulsionPinAssignments {
    l298n_en_a_pin: u32,
    l298n_en_b_pin: u32,
    l298n_in_1_pin: u32,
    l298n_in_2_pin: u32,
    l298n_in_3_pin: u32,
    l298n_in_4_pin: u32,
}

pub struct PropulsionControllerInstances {
    gpio: Chip,
    lmtr_en_a: Pwm,
    rmtr_en_b: Pwm,
}

pub struct Propulsion {
    left_wheel: WheelState,
    right_wheel: WheelState,
    pin_controller_instances: PropulsionControllerInstances,
    #[cfg(hardware)]
    pin_assignments: PropulsionPinAssignments,
}

impl Freezable for Propulsion {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.left_wheel, encoder)?;
        Encode::encode(&self.right_wheel, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.left_wheel = Decode::decode(decoder)?;
        self.right_wheel = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuSinkTask for Propulsion {
    type Input<'m> = input_msg!(PropulsionPayload);

    fn new(config: Option<&ComponentConfig>) -> Result<Self, CuError>
    where Self: Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        let l298n_en_a_pin_offset: u32 = kv
            .get("l298n_en_a")
            .expect("l298n_en_a for Propulsion not set in RON config")
            .clone()
            .into();

        let l298n_en_b_pin_offset: u32 = kv
            .get("l298n_en_b")
            .expect("l298n_en_b for Propulsion not set in RON config")
            .clone()
            .into();

        let l298n_in_1_pin_offset: u32 = kv
            .get("l298n_in_1")
            .expect("l298n_in_1 for Propulsion not set in RON config")
            .clone()
            .into();

        let l298n_in_2_pin_offset: u32 = kv
            .get("l298n_in_2")
            .expect("l298n_in_2 for Propulsion not set in RON config")
            .clone()
            .into();

        let l298n_in_3_pin_offset: u32 = kv
            .get("l298n_in_3")
            .expect("l298n_in_3 for Propulsion not set in RON config")
            .clone()
            .into();

        let l298n_in_4_pin_offset: u32 = kv
            .get("l298n_in_4")
            .expect("l298n_in_4 for Propulsion not set in RON config")
            .clone()
            .into();

        #[cfg(hardware)]
        let lmtr_en_a_instance = Pwm::new(0, l298n_en_a_pin_offset).unwrap();
        #[cfg(hardware)]
        let rmtr_en_b_instance = Pwm::new(0, l298n_en_b_pin_offset).unwrap();
        #[cfg(hardware)]
        let mut gpio = Chip::new("/dev/gpiochip4").unwrap();

        let pin_assignments = PropulsionPinAssignments {
            l298n_en_a_pin: l298n_en_a_pin_offset,
            l298n_en_b_pin: l298n_en_b_pin_offset,
            l298n_in_1_pin: l298n_in_1_pin_offset,
            l298n_in_2_pin: l298n_in_2_pin_offset,
            l298n_in_3_pin: l298n_in_3_pin_offset,
            l298n_in_4_pin: l298n_in_4_pin_offset
        };

        let pin_controller_instances = PropulsionControllerInstances {
            gpio: gpio,
            lmtr_en_a: lmtr_en_a_instance,
            rmtr_en_b: rmtr_en_b_instance,
        };

        Ok(Self {
            left_wheel: WheelState::default(),
            right_wheel: WheelState::default(),
            pin_controller_instances: pin_controller_instances,
            pin_assignments: pin_assignments,
        })
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
