use dumb_ina219::{units::{CurrentUnit, Gettable, ResistanceUnit}, *};

const TARGET_ADDR: u8 = 0x44;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dev = Ina219::new(
        ResistanceUnit::milliohms(100.0),
        CurrentUnit::milliamps(1000.0),
        TARGET_ADDR)?;
    dev.init()?;
    let current_reading = dev.load_current()?;
    let shunt_voltage_reading = dev.shunt_voltage()?;
    let bus_voltage_reading = dev.bus_voltage()?;
    let power_reading = dev.power()?;


    println!("Load current: {:?} mA", current_reading.get_val()*1000.0);
    println!("Shunt voltage: {:?} mV", shunt_voltage_reading.get_val()*1000.0);
    println!("Bus voltage: {:?} V", bus_voltage_reading.get_val());
    println!("Power: {:?} mW", power_reading.get_val()*1000.0);
    Ok(())
}
