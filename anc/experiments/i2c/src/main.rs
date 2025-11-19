use dumb_ina219::{units::{CurrentUnit, Gettable, ResistanceUnit}, *};

// 0x40 - Default - A0 OPEN, A1 OPEN
// 0x41 - A0 CLOSE, A1 OPEN
// 0x44 - A0 OPEN, A1 CLOSE
const TARGET_ADDR: u8 = 0x44;
// const TARGET_ADDR_2: u8 = 0x40;

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

    // let mut dev_2 = Ina219::new(
    //     ResistanceUnit::milliohms(100.0),
    //     CurrentUnit::milliamps(1000.0),
    //     TARGET_ADDR_2)?;
    // dev_2.init()?;
    // let current_reading_2 = dev_2.load_current()?;
    // let shunt_voltage_reading_2 = dev_2.shunt_voltage()?;
    // let bus_voltage_reading_2 = dev_2.bus_voltage()?;
    // let power_reading_2 = dev_2.power()?;


    println!("Load current: {:?} mA", current_reading.get_val()*1000.0);
    println!("Shunt voltage: {:?} mV", shunt_voltage_reading.get_val()*1000.0);
    println!("Bus voltage: {:?} V", bus_voltage_reading.get_val());
    println!("Power: {:?} mW", power_reading.get_val()*1000.0);

    // println!("\nLoad current 2: {:?} mA", current_reading_2.get_val()*1000.0);
    // println!("Shunt voltage 2: {:?} mV", shunt_voltage_reading_2.get_val()*1000.0);
    // println!("Bus voltage 2: {:?} V", bus_voltage_reading_2.get_val());
    // println!("Power 2: {:?} mW", power_reading_2.get_val()*1000.0);
    Ok(())
}
