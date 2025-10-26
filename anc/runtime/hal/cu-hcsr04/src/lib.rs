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
    input_pins: &[Line],
    output_pins: &[Line],

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
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        // let parsed_input_offsets = match kv.get("input_pins") {
        //     Some(pin_arr) => {
        //         Ok(Vec::<u32>::from(pin_arr.clone()))
        //     },
        //     Some(val) => panic!("{} is not an array.", val),
        //     None => None
        // };

        let input_pins_offsets: Option<Vec<u32>> = match kv.get("input_pins") {
            Some(val) => val.clone().into(),
            None => None
        };

        let input_pins_offsets: u32 = kv.get("input_pins").unwrap().clone().into();

        // let input_pins_offsets: Vec<u32> = kv
        //     .get("input_pins")
        //     .and_then(|pin_arr| pin_arr.as_array())
        //     .ok_or("input_pins in RON need to be a RON list")?
        //     .iter()
        //     .filter_map(|pin| pin.as_integer())
        //     .map(|pin| pin as u32)
        //     .collect();

        // {
        //     Some(output_pins) => Some(output_pins.clone().into()),
        //     None => None
        // };

        let output_pins_offsets: Option<u32> = match kv.get("output_pins") {
            Some(output_pins) => Some(output_pins.clone().into()),
            None => None
        };

        #[cfg(hardware)]
        let mut input_pin_handles = gpio()
            .get_lines(input_pins_offsets)
            .expect("GPIO line error, pin number invalid")
            .request(LineRequestFlags::INPUT, 0, consumer);

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
