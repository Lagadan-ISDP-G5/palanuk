extern crate cu_bincode as bincode;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

impl Freezable for ItpMerger {}

/// IMPORTANT: The #[serde(transparent)] is so that rmp_serde treats these tuple structs as the raw
/// types they contain, so that from_slice::<S>() in cu-zenoh-src will decode the primitive type sent from
/// the wire directly

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[derive(Reflect)]
pub struct ItpAccelerateCmd(pub u8);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
#[derive(Reflect)]
#[reflect(no_field_bounds, from_reflect = false)]
pub struct ItpTopicsOutputPayload {
    pub accelerate_cmd: bool
}

#[derive(Reflect)]
#[reflect(no_field_bounds, from_reflect = false)]
pub struct ItpMerger {
    last_cmd: bool
}

impl CuTask for ItpMerger {
    // u8 - itp_accelerate_cmd

    type Input<'m>
    = input_msg!('m,
            ItpAccelerateCmd
        );
    type Output<'m> = output_msg!(ItpTopicsOutputPayload);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
        where
            Self: Sized {
        Ok(Self { last_cmd: false })
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        if let Some(itp_accelerate_cmd) = input.payload()
        {
            let cmd = match itp_accelerate_cmd.0 {
                0 => false,
                1 => true,
                _ => false
            };

            let final_cmd = match (self.last_cmd, cmd) {
                (false, true) => true,
                _ => false
            };

            self.last_cmd = cmd;

            output.set_payload(
                ItpTopicsOutputPayload {
                    accelerate_cmd: final_cmd
                }
            );
        }

        Ok(())
    }

}
