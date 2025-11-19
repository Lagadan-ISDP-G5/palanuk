use std::panic;

use dumb_ina219::{units::{CurrentUnit, Gettable, PowerUnit, ResistanceUnit, VoltageUnit}, *};
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub struct CuIna219 {
    driver_instance: Ina219,
    target_addr: u8,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Serialize, Deserialize)]
pub struct Ina219Payload {
    pub power: f64,
    pub load_current: f64,
    pub shunt_voltage: f64,
    pub bus_voltage: f64,
    pub target_addr: u8,
}

// Sensor is stateless
impl Freezable for CuIna219 {}

impl CuSrcTask for CuIna219 {
    type Output<'m> = output_msg!(Ina219Payload);
    fn new(config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        let target_addr: u8 = kv
            .get("target_addr")
            .expect("I2C target address for an INA219 not set in RON config")
            .clone()
            .into();

        let driver_instance = Ina219::new(
            ResistanceUnit::milliohms(100.0),
            CurrentUnit::milliamps(1000.0),
            target_addr).ok();

        // Do not forget to call init() !
        match driver_instance {
            None => panic!("INA219 driver instantiation error"),
            Some(mut driver_instance) => {
                match driver_instance.init().ok() {
                    Some(_) => return Ok(Self { driver_instance, target_addr }),
                    None => panic!("INA219 init error (instantiation successful)")
                }
            }
        }
    }

    fn process(&mut self, _clock: &RobotClock, msg: &mut Self::Output<'_>) -> CuResult<()> {
        let dev = &mut self.driver_instance;

        let power_reading = match dev.power().ok() {
            Some(val) => val,
            None => PowerUnit::milliwatts(0.0)
        };

        let current_reading = match dev.load_current().ok() {
            Some(val) => val,
            None => CurrentUnit::milliamps(0.0)
        };

        let shunt_voltage_reading = match dev.shunt_voltage().ok() {
            Some(val) => val,
            None => VoltageUnit::millivolts(0.0)
        };

        let bus_voltage_reading = match dev.bus_voltage().ok() {
            Some(val) => val,
            None => VoltageUnit::millivolts(0.0)
        };

        msg.set_payload(
            Ina219Payload {
                power           : power_reading.get_val(),
                load_current    : current_reading.get_val(),
                shunt_voltage   : shunt_voltage_reading.get_val(),
                bus_voltage     : bus_voltage_reading.get_val(),
                target_addr     : self.target_addr
            }
        );
        let power_preview = power_reading.get_val()*1000.0;
        msg.metadata.set_status(format!("{power_preview:.2} mW"));
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
