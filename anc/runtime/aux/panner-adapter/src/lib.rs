extern crate cu_bincode as bincode;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use cu_cam_pan::{CameraPanningPayload, PositionCommand};
use propulsion_adapter::PropulsionAdapterOutputPayload;

pub struct PannerAdapter {
    cmd: PositionCommand
}

impl Freezable for PannerAdapter {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.cmd, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.cmd = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuTask for PannerAdapter {
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload);
    type Output<'m> = output_msg!(CameraPanningPayload);
    type Resources<'r> = ();

    fn new(_config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self { cmd: PositionCommand::default() })
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let msg = input.payload().map_or(Err(CuError::from(format!("none pload PannerAdapter"))), |msg| {Ok(msg)})?;
        output.set_payload(msg.panner_payload);
        Ok(())
    }
}
