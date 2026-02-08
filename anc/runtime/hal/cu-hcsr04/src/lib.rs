extern crate cu_bincode as bincode;

use cu29::prelude::*;
use bincode::{Decode, Encode};
use hcsr04_gpio_cdev::*;

use serde::{Deserialize, Serialize};

pub struct CuHcSr04 {
    driver_instance: HcSr04,
    last_value: Option<HcSr04Payload>,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Serialize, Deserialize)]
pub struct HcSr04Payload {
    pub distance: f64,
}

impl Freezable for CuHcSr04 {}

impl CuSrcTask for CuHcSr04 {
    type Output<'m> = output_msg!(HcSr04Payload);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>,  _resources: Self::Resources<'_>) -> CuResult<Self>
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
            driver_instance,
            last_value: None,
        })
    }

    fn process(&mut self, _clock: &RobotClock, output: &mut Self::Output<'_>) -> CuResult<()> {
        let dist_cm = self.driver_instance.dist_cm(None);

        // Update last_value on successful reading
        if let Ok(dist) = dist_cm {
            self.last_value = Some(HcSr04Payload { distance: dist.to_val() });
        }

        // Always output last_value if we have one (sticky behavior)
        if let Some(payload) = self.last_value {
            output.set_payload(payload);
            output.metadata.set_status(format!("{:.2} cm", payload.distance));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // leave it to an LLM later
}
