use std::panic;

extern crate cu_bincode as bincode;
use dumb_ina219::{units::{CurrentUnit, Gettable, ResistanceUnit}, *};
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
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
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
                    None => panic!("INA219 init error (instantiation successful). Check I2C wiring.")
                }
            }
        }
    }

    fn process(&mut self, _clock: &RobotClock, msg: &mut Self::Output<'_>) -> CuResult<()> {
        let dev = &mut self.driver_instance;

        let power_reading = dev.power().map_err(|_| {
            CuError::from(format!("failed to get power reading"))
        })?;

        let current_reading = dev.load_current().map_err(|_| {
            CuError::from(format!("failed to get current reading"))
        })?;

        let shunt_voltage_reading = dev.shunt_voltage().map_err(|_| {
            CuError::from(format!("failed to get shunt voltage reading"))
        })?;

        let bus_voltage_reading = dev.bus_voltage().map_err(|_| {
            CuError::from(format!("failed to get bus voltage reading"))
        })?;

        let power = power_reading.get_val()*1000.0;
        let current = current_reading.get_val()*1000.0;
        let shunt_voltage = shunt_voltage_reading.get_val()*1000.0;
        let bus_voltage = bus_voltage_reading.get_val();

        msg.set_payload(
            Ina219Payload {
                power           : power,
                load_current    : current,
                shunt_voltage   : shunt_voltage,
                bus_voltage     : bus_voltage,
                target_addr     : self.target_addr
            }
        );
        // msg.metadata.set_status(format!("{power:.2}mW {current:.2}mA {bus_voltage:.2}V"));
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
