use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(hardware)]
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

pub struct RPGpio {
    #[cfg(hardware)]
    pin: Line,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Serialize, Deserialize)]
pub struct RPGpioPayload {
    pub on: bool,
}

#[cfg(hardware)]
impl From<RPGpioPayload> for bool {
    fn from(msg: RPGpioPayload) -> Self {
        msg.on
    }
}

#[cfg(hardware)]
impl From<RPGpioPayload> for u8 {
    fn from(msg: RPGpioPayload) -> Self {
        if msg.on {
            1
        } else {
            0
        }
    }
}

impl Freezable for RPGpio {}

impl CuSinkTask for RPGpio {
    type Input<'m> = input_msg!(RPGpioPayload);

    fn new(config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self:Sized, {
        // ...
        Ok(())
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
