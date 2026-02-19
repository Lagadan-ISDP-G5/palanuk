extern crate cu_bincode as bincode;
use dumb_sysfs_pwm::{Pwm, PwmBuilder};
use gpio_cdev::*;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub struct DirectionPair(u8, u8);
// Just reassign these if the actual hardware connections happen to be flipped
const FORWARD: DirectionPair = DirectionPair(1, 0);
const BACKWARDS: DirectionPair = DirectionPair(0, 1);
const STOP: DirectionPair = DirectionPair(0, 0);

/// ReallySlow by default
#[derive(Debug, PartialEq, Eq, Default)]
pub enum Speed {
    #[default]
    ReallySlow,
    Slow,
    NotSlow,
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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub enum WheelDirection {
    Forward,
    Reverse,
    #[default]
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
    #[allow(unused)]
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
    #[allow(unused)]
    pin_assignments: PropulsionPinAssignments,
    last_lmtr_duty_cycle: Option<f32>,
    last_rmtr_duty_cycle: Option<f32>,
    period_ns: u32
}

impl Freezable for Propulsion {
    fn freeze<E: cu_bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), cu_bincode::error::EncodeError> {
        Encode::encode(&self.left_wheel, encoder)?;
        Encode::encode(&self.right_wheel, encoder)?;
        Ok(())
    }

    fn thaw<D: cu_bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), cu_bincode::error::DecodeError> {
        self.left_wheel = Decode::decode(decoder)?;
        self.right_wheel = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuSinkTask for Propulsion {
    type Input<'m> = input_msg!(PropulsionPayload);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> Result<Self, CuError>
    where Self: Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        let period_ns: u32 = kv
            .get("period_ns")
            .map_or(20_000_000, |p: &config::Value| -> u32 {p.clone().into()})
            .into();

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

        let lmtr_en_a_instance = PwmBuilder::new(0, l298n_en_a_pin_offset, 20_000_000).build().unwrap();
        let rmtr_en_b_instance = PwmBuilder::new(0, l298n_en_b_pin_offset, 20_000_000).build().unwrap();
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
            last_lmtr_duty_cycle: None,
            last_rmtr_duty_cycle: None,
            period_ns
        })
    }

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        let en_a_hdl = &mut self.pin_controller_instances.lmtr_en_a;
        let en_b_hdl = &mut self.pin_controller_instances.rmtr_en_b;

        en_a_hdl.set_period_ns(self.period_ns).unwrap();
        en_b_hdl.set_period_ns(self.period_ns).unwrap();

        match en_a_hdl.set_duty_cycle(0.0) {
            Ok(_) => (),
            Err(e) => {return Err(CuError::from(format!("Failed to init propulsion Pwm: {}", e.to_string())))}
        }

        match en_b_hdl.set_duty_cycle(0.0) {
            Ok(_) => (),
            Err(e) => {return Err(CuError::from(format!("Failed to init propulsion Pwm: {}", e.to_string())))}
        }

        Ok(())
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>) -> Result<(), CuError> {
        let en_a_hdl = &mut self.pin_controller_instances.lmtr_en_a;
        let en_a_is_enabled = en_a_hdl.get_enable();

        let en_b_hdl = &mut self.pin_controller_instances.rmtr_en_b;
        let en_b_is_enabled = en_b_hdl.get_enable();

        let payload = input.payload();

        if let Some(payload) = payload {
            if payload.left_enable {
                self.left_wheel.enable = true;
                match en_a_is_enabled {
                    true => (),
                    false => {
                        match en_a_hdl.set_enable(true) {
                            Ok(_) => (),
                            Err(_) => return Err(CuError::from(format!("Failed to set enable")))
                        }
                    }
                }
            }
            else {
                self.left_wheel.enable = false;
                match en_a_is_enabled {
                    true => {
                        match en_a_hdl.set_enable(false) {
                            Ok(_) => (),
                            Err(_) => return Err(CuError::from(format!("Failed to set enable")))
                        }
                    },
                    false => ()
                }
            }

            if payload.right_enable {
                self.right_wheel.enable = true;
                match en_b_is_enabled {
                    true => (),
                    false => {
                        match en_b_hdl.set_enable(true) {
                            Ok(_) => (),
                            Err(_) => return Err(CuError::from(format!("Failed to set enable")))
                        }
                    }
                }
            }
            else {
                self.right_wheel.enable = false;
                match en_b_is_enabled {
                    true => {
                        match en_b_hdl.set_enable(false) {
                            Ok(_) => (),
                            Err(_) => return Err(CuError::from(format!("Failed to set enable")))
                        }
                    },
                    false => ()
                }
            }
            self.last_lmtr_duty_cycle = Some(payload.left_speed);
            self.last_rmtr_duty_cycle = Some(payload.right_speed);

            let dir_hdl = &mut self.pin_controller_instances.direction_pins;

            let in_1_line = &dir_hdl.in_1_pin;
            let in_2_line = &dir_hdl.in_2_pin;
            let in_3_line = &dir_hdl.in_3_pin;
            let in_4_line = &dir_hdl.in_4_pin;

            match payload.left_direction {
                WheelDirection::Forward => {
                    let DirectionPair(in_1_val, in_2_val) = FORWARD;
                    let (ret1, ret2) = (in_1_line.set_value(in_1_val), in_2_line.set_value(in_2_val));
                    if let Ok(_) = ret1 && let Ok(_) = ret2 {}
                    else {
                        return Err(CuError::from(format!("Failed to set direction")))
                    }
                },
                WheelDirection::Reverse => {
                    let DirectionPair(in_1_val, in_2_val) = BACKWARDS;
                    let (ret1, ret2) = (in_1_line.set_value(in_1_val), in_2_line.set_value(in_2_val));
                    if let Ok(_) = ret1 && let Ok(_) = ret2 {}
                    else {
                        return Err(CuError::from(format!("Failed to set direction")))
                    }
                },
                WheelDirection::Stop => {
                    let DirectionPair(in_1_val, in_2_val) = STOP;
                    let (ret1, ret2) = (in_1_line.set_value(in_1_val), in_2_line.set_value(in_2_val));
                    if let Ok(_) = ret1 && let Ok(_) = ret2 {}
                    else {
                        return Err(CuError::from(format!("Failed to set direction")))
                    }
                }
            }

            // Right wheel seems to be flipped
            match payload.right_direction {
                WheelDirection::Forward => {
                    let DirectionPair(in_3_val, in_4_val) = FORWARD;
                    let (ret1, ret2) = (in_3_line.set_value(in_4_val), in_4_line.set_value(in_3_val));
                    if let Ok(_) = ret1 && let Ok(_) = ret2 {}
                    else {
                        return Err(CuError::from(format!("Failed to set direction")))
                    }
                },
                WheelDirection::Reverse => {
                    let DirectionPair(in_3_val, in_4_val) = BACKWARDS;
                    let (ret1, ret2) = (in_3_line.set_value(in_4_val), in_4_line.set_value(in_3_val));
                    if let Ok(_) = ret1 && let Ok(_) = ret2 {}
                    else {
                        return Err(CuError::from(format!("Failed to set direction")))
                    }
                },
                WheelDirection::Stop => {
                    let DirectionPair(in_3_val, in_4_val) = STOP;
                    let (ret1, ret2) = (in_3_line.set_value(in_4_val), in_4_line.set_value(in_3_val));
                    if let Ok(_) = ret1 && let Ok(_) = ret2 {}
                    else {
                        return Err(CuError::from(format!("Failed to set direction")))
                    }
                }
            }

        }

        match en_a_hdl.set_duty_cycle(self.last_lmtr_duty_cycle.unwrap_or(0.0).clamp(0.0, 1.0)) {
            Ok(_) => (),
            Err(_) => return Err(CuError::from(format!("Failed to set duty cycle")))
        };

        match en_b_hdl.set_duty_cycle(self.last_rmtr_duty_cycle.unwrap_or(0.0).clamp(0.0, 1.0)) {
            Ok(_) => (),
            Err(_) => return Err(CuError::from(format!("Failed to set duty cycle")))
        };

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

        let line_1_ret = in_1_line.set_value(0).ok();
        let line_2_ret = in_2_line.set_value(0).ok();
        let line_3_ret = in_3_line.set_value(0).ok();
        let line_4_ret = in_4_line.set_value(0).ok();
        let mut stop_success: bool = false;

        if
            let Some(_) = line_1_ret &&
            let Some(_) = line_2_ret &&
            let Some(_) = line_3_ret &&
            let Some(_) = line_4_ret
        {
            let (ret1, ret2) = (en_a_hdl.set_duty_cycle(0.0), en_b_hdl.set_duty_cycle(0.0));
            if let Ok(_) = ret1 && let Ok(_) = ret2 {
                let (ret1, ret2) = (en_a_hdl.unexport(), en_b_hdl.unexport());
                if let Ok(_) = ret1 && let Ok(_) = ret2 {
                    stop_success = true;
                }
            }
        }

        match stop_success {
            true => Ok(()),
            false => Err(CuError::from(format!("Failed to stop cu-propulsion due to I/O error")))
        }
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
