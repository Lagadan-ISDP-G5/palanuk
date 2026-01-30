/// Zenoh publisher task for topics under EC

use cu29::prelude::*;
use cu_powermon::Ina219Payload;

pub struct EcPub {}

impl Freezable for EcPub {}

impl CuTask for EcPub {
    type Input<'m> = input_msg!('m, Ina219Payload);
    // f64 - ec_power_mwatts
    // f64 - ec_load_current_mamps
    // f64 - ec_bus_voltage_mvolts
    // f64 - ec_shunt_voltage_mvolts

    type Output<'m> = output_msg!((f64, f64, f64, f64));

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
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

            output.set_payload((
                power,
                load_current,
                bus_voltage,
                shunt_voltage
            ));
        }

        Ok(())
    }
}
