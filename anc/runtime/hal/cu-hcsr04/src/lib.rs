use cu29::prelude::*;
use bincode::{Decode, Encode};
use hcsr04_gpio_cdev::*;

use serde::{Deserialize, Serialize};

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

        let dist_threshold_cm: u32 = kv
            .get("dist_threshold_cm")
            .expect("threshold for HcSr04 not set in RON config")
            .clone()
            .into();

        let driver_instance = HcSr04::new(trig_pin_offset, echo_pin_offset, DistanceUnit::Cm(dist_threshold_cm as f64)).expect("GPIO driver error");

        Ok(Self {
            driver_instance
        })
    }

    fn process(&mut self, _clock: &RobotClock, msg: &mut Self::Output<'_>) -> CuResult<()> {
        let dist_cm = self.driver_instance.dist_cm(None);

        let dist_msg = dist_cm.map_err(|e| {
            match e {
                HcSr04Error::Init => CuError::from(format!("hcsr04 init")),
                HcSr04Error::Io => CuError::from(format!("echo/trig failure")),
                HcSr04Error::LineEventHandleRequest => CuError::from(format!("line event req")),
                HcSr04Error::PollFd => CuError::from(format!("fd polling"))
            }
        })?.to_val();

        msg.set_payload(HcSr04Payload { distance: dist_msg });
        msg.metadata.set_status(format!("{dist_msg:.2} cm"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // leave it to an LLM later
}
