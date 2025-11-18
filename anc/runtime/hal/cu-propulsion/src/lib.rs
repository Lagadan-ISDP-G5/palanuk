use dumb_sysfs_pwm::Pwm;
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

pub struct DirectionPair(u8, u8);
// Just reassign these if the actual hardware connections happen to be flipped
const FORWARD: DirectionPair = DirectionPair(0, 1);
const BACKWARDS: DirectionPair = DirectionPair(1, 0);
const STOP: DirectionPair = DirectionPair(0, 0);

/// ReallySlow by default
#[derive(Debug, PartialEq, Eq, Default)]
pub enum Speed {
    #[default]
    ReallySlow,
    Slow,
    NotSlow,
}

// Might remove this abstraction in the future, it might also be useful
// For now speed in the payload is just the duty cycle
const REALLY_SLOW: f32 = 0.2;
const SLOW: f32 = 0.4;
const NOT_SLOW: f32 = 0.8;

impl Speed {
    fn get_duty_cycle(&self, speed: Speed) -> f32 {
        match speed {
            Speed::ReallySlow => REALLY_SLOW,
            Speed::Slow => SLOW,
            Speed::NotSlow => NOT_SLOW
        }
    }
}

/// `left_speed` and `right_speed` are the percentage duty cycles for the Pwm controllers of each wheel.
#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct PropulsionPayload {
    pub left_enable: bool,
    pub right_enable: bool,
    pub left_speed: f32,
    pub right_speed: f32,
    pub left_direction: WheelDirection,
    pub right_direction: WheelDirection,
    // active_cfg: PropulsionPinAssignments,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub enum WheelDirection {
    #[default]
    Forward,
    Reverse,
    Stop
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct WheelState {
    enable: bool,
    direction: WheelDirection,
    speed: f64,
}

impl WheelState {
    fn default() -> Self {
        Self { enable: false, direction: WheelDirection::Stop, speed: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct PropulsionPinAssignments {
    l298n_en_a_pin: u32,
    l298n_en_b_pin: u32,
    l298n_in_1_pin: u32,
    l298n_in_2_pin: u32,
    l298n_in_3_pin: u32,
    l298n_in_4_pin: u32,
}

pub struct PropulsionControllerInstances {
    gpio_inst: Chip,
    direction_pins: DirectionPinHdls,
    lmtr_en_a: Pwm,
    rmtr_en_b: Pwm,
}

struct DirectionPinHdls {
    in_1_pin: LineHandle,
    in_2_pin: LineHandle,
    in_3_pin: LineHandle,
    in_4_pin: LineHandle,
}

pub struct Propulsion {
    left_wheel: WheelState,
    right_wheel: WheelState,
    pin_controller_instances: PropulsionControllerInstances,
    // #[cfg(hardware)]
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
            .expect("l298n_en_a for Propulsion not set in RON config. Make sure you're specifying the PWM channel offset instead of its GPIO number.")
            .clone()
            .into();

        let l298n_en_b_pin_offset: u32 = kv
            .get("l298n_en_b")
            .expect("l298n_en_b for Propulsion not set in RON config. Make sure you're specifying the PWM channel offset instead of its GPIO number.")
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


        let lmtr_en_a_instance = Pwm::new(0, l298n_en_a_pin_offset).unwrap();
        let rmtr_en_b_instance = Pwm::new(0, l298n_en_b_pin_offset).unwrap();
        let mut gpio = Chip::new("/dev/gpiochip4").unwrap();

        let pin_assignments = PropulsionPinAssignments {
            l298n_en_a_pin: l298n_en_a_pin_offset,
            l298n_en_b_pin: l298n_en_b_pin_offset,
            l298n_in_1_pin: l298n_in_1_pin_offset,
            l298n_in_2_pin: l298n_in_2_pin_offset,
            l298n_in_3_pin: l298n_in_3_pin_offset,
            l298n_in_4_pin: l298n_in_4_pin_offset
        };

        let in_1_line = gpio.get_line(l298n_in_1_pin_offset).unwrap();
        let in_1_line = in_1_line.request(LineRequestFlags::OUTPUT, 0, "in-1-left-motor").unwrap();

        let in_2_line = gpio.get_line(l298n_in_2_pin_offset).unwrap();
        let in_2_line = in_2_line.request(LineRequestFlags::OUTPUT, 0, "in-2-left-motor").unwrap();

        let in_3_line = gpio.get_line(l298n_in_3_pin_offset).unwrap();
        let in_3_line = in_3_line.request(LineRequestFlags::OUTPUT, 0, "in-3-right-motor").unwrap();

        let in_4_line = gpio.get_line(l298n_in_4_pin_offset).unwrap();
        let in_4_line = in_4_line.request(LineRequestFlags::OUTPUT, 0, "in-4-right-motor").unwrap();


        let pin_controller_instances = PropulsionControllerInstances {
            gpio_inst: gpio,
            direction_pins: DirectionPinHdls {
                                in_1_pin: in_1_line,
                                in_2_pin: in_2_line,
                                in_3_pin: in_3_line,
                                in_4_pin: in_4_line
                            },
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

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        let en_a_hdl = &mut self.pin_controller_instances.lmtr_en_a;
        let en_b_hdl = &mut self.pin_controller_instances.rmtr_en_b;

        _ = en_a_hdl.export();
        _ = en_b_hdl.export();

        _ = en_a_hdl.set_period_ns(20_000);
        _ = en_b_hdl.set_period_ns(20_000);

        _ = en_a_hdl.set_duty_cycle(0.0);
        _ = en_b_hdl.set_duty_cycle(0.0);
        Ok(())
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>) -> Result<(), CuError> {
        let en_a_hdl = &mut self.pin_controller_instances.lmtr_en_a;
        let en_b_hdl = &mut self.pin_controller_instances.rmtr_en_b;

        let payload = input.payload().unwrap();
        if payload.left_enable {
            self.left_wheel.enable = true;
            match en_a_hdl.get_enabled().unwrap() {
                true => (),
                false => en_a_hdl.enable(true).unwrap()
            }
        }
        else {
            self.left_wheel.enable = false;
            match en_a_hdl.get_enabled() {
                Ok(true) => {
                    en_a_hdl.enable(false).unwrap();
                },
                Ok(false) => (),
                Err(_) => ()
            }
        }

        if payload.right_enable {
            self.right_wheel.enable = true;
            match en_b_hdl.get_enabled().unwrap() {
                true => (),
                false => en_b_hdl.enable(true).unwrap()
            }
        }
        else {
            self.right_wheel.enable = false;
            match en_b_hdl.get_enabled() {
                Ok(true) => {
                    en_b_hdl.enable(false).unwrap();
                },
                Ok(false) => (),
                Err(_) => ()
            }
        }

        en_a_hdl.set_duty_cycle(payload.left_speed).unwrap();
        en_b_hdl.set_duty_cycle(payload.right_speed).unwrap();

        let dir_hdl = &mut self.pin_controller_instances.direction_pins;

        let in_1_line = &dir_hdl.in_1_pin;
        let in_2_line = &dir_hdl.in_2_pin;
        let in_3_line = &dir_hdl.in_3_pin;
        let in_4_line = &dir_hdl.in_4_pin;

        match payload.left_direction {
            WheelDirection::Forward => {
                let DirectionPair(in_1_val, in_2_val) = FORWARD;
                in_1_line.set_value(in_1_val).unwrap();
                in_2_line.set_value(in_2_val).unwrap();
            },
            WheelDirection::Reverse => {
                let DirectionPair(in_1_val, in_2_val) = BACKWARDS;
                in_1_line.set_value(in_1_val).unwrap();
                in_2_line.set_value(in_2_val).unwrap();
            },
            WheelDirection::Stop => {
                let DirectionPair(in_1_val, in_2_val) = STOP;
                in_1_line.set_value(in_1_val).unwrap();
                in_2_line.set_value(in_2_val).unwrap();
            }
        }

        // Right wheel seems to be flipped
        match payload.right_direction {
            WheelDirection::Forward => {
                let DirectionPair(in_3_val, in_4_val) = FORWARD;
                in_3_line.set_value(in_4_val).unwrap();
                in_4_line.set_value(in_3_val).unwrap();
            },
            WheelDirection::Reverse => {
                let DirectionPair(in_3_val, in_4_val) = BACKWARDS;
                in_3_line.set_value(in_4_val).unwrap();
                in_4_line.set_value(in_3_val).unwrap();
            },
            WheelDirection::Stop => {
                let DirectionPair(in_3_val, in_4_val) = STOP;
                in_3_line.set_value(in_4_val).unwrap();
                in_4_line.set_value(in_3_val).unwrap();
            }
        }
        Ok(())
    }

    fn stop(&mut self, _clock: &RobotClock) -> CuResult<()> {
        let dir_hdl = &mut self.pin_controller_instances.direction_pins;
        let in_1_line = &dir_hdl.in_1_pin;
        let in_2_line = &dir_hdl.in_2_pin;
        let in_3_line = &dir_hdl.in_3_pin;
        let in_4_line = &dir_hdl.in_4_pin;

        let en_a_hdl = &mut self.pin_controller_instances.lmtr_en_a;
        let en_b_hdl = &mut self.pin_controller_instances.rmtr_en_b;

        in_1_line.set_value(0).unwrap();
        in_2_line.set_value(0).unwrap();
        in_3_line.set_value(0).unwrap();
        in_4_line.set_value(0).unwrap();

        _ = en_a_hdl.set_duty_cycle(0.0);
        _ = en_b_hdl.set_duty_cycle(0.0);

        _ = en_a_hdl.unexport();
        _ = en_b_hdl.unexport();
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
