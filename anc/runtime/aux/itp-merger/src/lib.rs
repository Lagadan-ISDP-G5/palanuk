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
#[serde(transparent)]
#[derive(Reflect)]
pub struct ItpBumpRockCmd(pub u8);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
#[derive(Reflect)]
#[reflect(no_field_bounds, from_reflect = false)]
pub struct ItpTopicsOutputPayload {
    pub accelerate_cmd: bool,
    pub bump_rock_cmd: bool,
}

#[derive(Reflect)]
#[reflect(no_field_bounds, from_reflect = false)]
pub struct ItpMerger {
    last_accel_cmd: bool,
    last_rock_cmd: bool,
}

impl CuTask for ItpMerger {
    // u8 - itp_accelerate_cmd

    type Input<'m>
    = input_msg!('m,
            ItpAccelerateCmd,
            ItpBumpRockCmd
        );
    type Output<'m> = output_msg!(ItpTopicsOutputPayload);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
        where
            Self: Sized {
        Ok(Self { last_accel_cmd: false, last_rock_cmd: false })
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        let (accel_input, rock_input) = *input;

        let accel_final = if let Some(itp_accelerate_cmd) = accel_input.payload() {
            let cmd = itp_accelerate_cmd.0 == 1;
            let rising = !self.last_accel_cmd && cmd;
            self.last_accel_cmd = cmd;
            rising
        } else {
            false
        };

        let rock_final = if let Some(itp_rock_cmd) = rock_input.payload() {
            let cmd = itp_rock_cmd.0 == 1;
            let rising = !self.last_rock_cmd && cmd;
            self.last_rock_cmd = cmd;
            rising
        } else {
            false
        };

        output.set_payload(
            ItpTopicsOutputPayload {
                accelerate_cmd: accel_final,
                bump_rock_cmd: rock_final,
            }
        );

        Ok(())
    }

}
