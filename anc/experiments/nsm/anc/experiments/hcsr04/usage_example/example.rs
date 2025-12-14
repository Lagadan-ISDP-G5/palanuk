use hcsr04::*;
use std::{thread::sleep, time::Duration};
const ECHO_PIN: u32 = 20; // GPIO20
const TRIG_PIN: u32 = 21; // GPIO21

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut hcsr04 = HcSr04::new(TRIG_PIN, ECHO_PIN)?;
    // let timeout = range_to_timeout(DistanceUnit::Cm(4.0))?;

    loop {
        let distance = hcsr04.dist_cm(None)?;
        println!("Distance: {:05.2}cm", distance.to_val());
        sleep(Duration::from_secs_f32(0.2));
    }
    Ok(())
}