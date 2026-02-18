extern crate cu_bincode as bincode;

use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use ir_encoder_gpio_cdev::*;

pub struct CuIrEncoder {
    lmtr_driver: IrEncoder,
    rmtr_driver: IrEncoder,
    last_value: Option<IrEncoderPayload>,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Serialize, Deserialize)]
pub struct IrEncoderPayload {
    pub lmtr_normalized_rpm: Option<f32>,
    pub rmtr_normalized_rpm: Option<f32>,
}

impl Freezable for CuIrEncoder {}

impl CuSrcTask for CuIrEncoder {
    type Output<'m> = output_msg!(IrEncoderPayload);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>,  _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self:Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        let lmtr_output_pin: u32 = kv
            .get("lmtr_output_pin")
            .expect("lmtr_output_pin for IrEncoder not set in RON config")
            .clone()
            .into();

        let rmtr_output_pin: u32 = kv
            .get("rmtr_output_pin")
            .expect("rmtr_output_pin for IrEncoder not set in RON config")
            .clone()
            .into();

        let num_of_slots: u32 = kv
            .get("num_of_slots")
            .expect("num_of_slots for IrEncoder not set in RON config")
            .clone()
            .into();

        let max_rpm: u32 = kv
            .get("max_rpm")
            .expect("max_rpm for IrEncoder not set in RON config")
            .clone()
            .into();

        let lmtr_driver = IrEncoder::new(lmtr_output_pin, Some(num_of_slots), Some(max_rpm)).expect("ir-encoder driver");
        let rmtr_driver = IrEncoder::new(rmtr_output_pin, Some(num_of_slots), Some(max_rpm)).expect("ir-encoder driver");

        Ok(Self {
            lmtr_driver,
            rmtr_driver,
            last_value: None
        })
    }

    fn process(&mut self, _clock: &RobotClock, output: &mut Self::Output<'_>) -> CuResult<()> {
        let lmtr_normalized_rpm = self.lmtr_driver.get_normalized_rpm();
        let rmtr_normalized_rpm = self.rmtr_driver.get_normalized_rpm();

        match (lmtr_normalized_rpm, rmtr_normalized_rpm) {
            (Ok(lmtr_rpm), Ok(rmtr_rpm)) => {
                self.last_value = Some(
                    IrEncoderPayload {
                        lmtr_normalized_rpm: Some(lmtr_rpm),
                        rmtr_normalized_rpm: Some(rmtr_rpm)
                    }
                )
            },
            _ => self.last_value = Some(
                IrEncoderPayload {
                    lmtr_normalized_rpm: None,
                    rmtr_normalized_rpm: None
                }
            )
        }

        if let Some(payload) = self.last_value {
            output.set_payload(payload);
        }

        Ok(())
    }
}
