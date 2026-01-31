
/// Zenoh publisher task for topics under EC

extern crate cu_bincode as bincode;
use cu29::prelude::*;
use cu_powermon::Ina219Payload;
use bincode::{Encode, Decode};
use serde::{Serialize, Deserialize};

pub struct EcPub {}

impl Freezable for EcPub {}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
pub struct PowerMwatts(pub f64);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
pub struct LoadCurrentMamps(pub f64);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
pub struct BusVoltageMvolts(pub f64);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
pub struct ShuntVoltageMvolts(pub f64);

impl CuTask for EcPub {
    type Input<'m> = input_msg!('m, Ina219Payload);
    // f64 - ec_power_mwatts
    // f64 - ec_load_current_mamps
    // f64 - ec_bus_voltage_mvolts
    // f64 - ec_shunt_voltage_mvolts

    type Output<'m> = output_msg!(PowerMwatts, LoadCurrentMamps, BusVoltageMvolts, ShuntVoltageMvolts);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        if let Some(ina219_payload)= input.payload() {
            let power = ina219_payload.power;
            let load_current = ina219_payload.load_current;
            let bus_voltage = ina219_payload.bus_voltage * 1000.0;
            let shunt_voltage = ina219_payload.shunt_voltage;

            output.0.set_payload(PowerMwatts(power));
            output.1.set_payload(LoadCurrentMamps(load_current));
            output.2.set_payload(BusVoltageMvolts(bus_voltage));
            output.3.set_payload(ShuntVoltageMvolts(shunt_voltage));
        }

        Ok(())
    }
}
