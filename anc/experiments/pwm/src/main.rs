use std::error::Error;
use std::time::Duration;
use rpi_pal::pwm::{Channel, Polarity, Pwm};

const PERIOD_NS: u64 = 1_000_000;

fn main() -> Result<(), Box<dyn Error>> {
    let pwm = Pwm::with_pwmchip(0, Channel::Pwm1 as u8)?; // GPIO13
    pwm.set_period(Duration::from_nanos(PERIOD_NS))?;
    pwm.set_polarity(Polarity::Normal)?;
    pwm.enable()?;

    for _ in 1..=12 {
        pwm.set_pulse_width(Duration::from_nanos(PERIOD_NS / 8))?;
        std::thread::sleep(Duration::from_millis(300));        
        pwm.set_pulse_width(Duration::from_nanos(PERIOD_NS / 4))?;
        std::thread::sleep(Duration::from_millis(300));
        pwm.set_pulse_width(Duration::from_nanos(PERIOD_NS / 2))?;
        std::thread::sleep(Duration::from_millis(300));
        pwm.set_pulse_width(Duration::from_nanos(PERIOD_NS))?;
        std::thread::sleep(Duration::from_millis(300));
    }

    Ok(())
    // When the pwm variable goes out of scope, the PWM channel is automatically disabled.
    // You can manually disable the channel by calling the Pwm::disable() method.
}