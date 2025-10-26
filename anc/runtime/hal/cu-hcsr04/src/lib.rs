use cu29::prelude::*;
use bincode::{Decode, Encode};

#[cfg(hardware)]
use serde::{Deserialize, Serialize};
use {
    gpio_cdev::*,
    std::sync::OnceLock
};

#[cfg(hardware)]
static GPIO: OnceLock<Chip> = OnceLock::new();

#[cfg(hardware)]
fn gpio() -> &'static Chip {
    GPIO.get_or_init(|| Chip::new("/dev/gpiochip4").expect("Failed to open /dev/gpiochip4"))
}

pub struct HcSr04 {
    #[cfg(hardware)]
    echo_pin: LineHandle,
    #[cfg(hardware)]
    trig_pin: LineHandle,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Serialize, Deserialize)]
pub struct HcSr04Payload {
    pub distance: f64,
}

impl Freezable for HcSr04 {}

impl CuSinkTask for HcSr04 {
    type Input<'m> = input_msg!(HcSr04Payload);

    fn new(config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self:Sized, {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        let trig_pin_offset: u32 = kv
            .get("trig_pin")
            .expect("trig_pin for HcSr04 not set in RON config")
            .clone()
            .into();

        let echo_pin_offset: u32 = kv
            .get("echo_pin")
            .expect("echo_pin for HcSr04 not set in RON config")
            .clone()
            .into();

        #[cfg(hardware)]
        let trig_pin_hndl = gpio()
        .get_line(trig_pin_offset)
        .expect("GPIO line error, trig_pin pin offset invalid")
        .request(LineRequestFlags::OUTPUT, 0, "hcsr04-trigger-pin")
        .expect("I/O error, check Pi GPIO userspace config");

        #[cfg(hardware)]
        let echo_pin_hndl = gpio()
        .get_line(echo_pin_offset)
        .expect("GPIO line error, echo_pin pin offset invalid")
        .request(LineRequestFlags::INPUT, 0, "hcsr04-echo-pin")
        .expect("I/O error, check Pi GPIO userspace config");

        Ok(Self {
            trig_pin: trig_pin_hndl,
            echo_pin: echo_pin_hndl
        })
    }

    fn process(&mut self, _clock: &RobotClock, msg: &Self::Input<'_>) -> CuResult<()> {
        // ...
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // leave it to an LLM later
}
