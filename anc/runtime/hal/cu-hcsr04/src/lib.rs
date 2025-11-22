use cu29::prelude::*;
use bincode::{Decode, Encode};
use hcsr04_gpio_cdev::*;

use serde::{Deserialize, Serialize};
pub const IGNORE_VAL: f64 = 69420.0;

pub struct CuHcSr04 {
    driver_instance: HcSr04
}

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Serialize, Deserialize)]
pub struct HcSr04Payload {
    pub distance: f64,
}

// Sensor is stateless
impl Freezable for CuHcSr04 {}

impl CuSrcTask for CuHcSr04 {
    type Output<'m> = output_msg!(HcSr04Payload);

    fn new(config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self:Sized
    {
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

        let driver_instance = HcSr04::new(trig_pin_offset, echo_pin_offset).expect("GPIO driver error");

        Ok(Self {
            driver_instance
        })
    }

    fn process(&mut self, _clock: &RobotClock, msg: &mut Self::Output<'_>) -> CuResult<()> {
        // let dist_cm = self.driver_instance.dist_cm(Some(range_to_timeout(DistanceUnit::Cm(100.0)).unwrap())).ok();
        let dist_cm = self.driver_instance.dist_cm(None).ok();

        let dist_msg = match dist_cm {
            Some(val) => val.to_val(),
            None => IGNORE_VAL
        };

        msg.set_payload(HcSr04Payload { distance: dist_msg });
        msg.metadata.set_status(format!("{dist_msg:.2} cm"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // leave it to an LLM later
}
