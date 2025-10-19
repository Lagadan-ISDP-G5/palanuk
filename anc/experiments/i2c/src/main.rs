extern crate ina219_rs as ina219;
extern crate linux_embedded_hal as hal;

use hal::I2cdev;
use ina219::physic::{self, ElectricCurrent};

use ina219::ina219::{INA219, Opts};

fn main() {
    let device = I2cdev::new("/dev/i2c-1").unwrap();
    let opt = Opts::new(0x44, 100 * physic::MilliOhm, 1 * physic::Ampere);
    let mut ina = INA219::new(device, opt);
    ina.init().unwrap();
    let pm = ina.sense().unwrap();
    println!("{:?}", pm);

    let i_raw = ina.current_raw().unwrap();
    println!("Raw current: {:?}", i_raw);
    let i_lsb = (1000 * physic::MilliAmpere) / ElectricCurrent::from(32768);
    println!("i_lsb: {}", i_lsb);
    let i_conv = (i_raw as i64) * i_lsb;
    println!(
        "Converted current: {:?}",
        (ElectricCurrent::from(i_conv) as f64) / (1000_i64.pow(2) as f64)
    );

    let p_raw = ina.power().unwrap();
    let p_lsb = 20 * i_lsb;
    let p_conv = p_raw * p_lsb;
}
