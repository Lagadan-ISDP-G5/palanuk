use cu29::prelude::*;
use iceoryx2::prelude::*;
use cu_propulsion::{PropulsionPayload};
use cu_cam_pan::{CameraPanningPayload};
use cu_hcsr04::{HcSr04Payload};
use cu_powermon::*;

pub struct Jogger {}
pub struct Panner {}

impl Freezable for Jogger {}
impl Freezable for Panner {}

impl CuTask for Jogger {
    type Input<'m> = input_msg!('m, HcSr04Payload);
    type Output<'m> = output_msg!(PropulsionPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        // use this method to init iox2 sub
    }

    fn process<'i, 'o>(&mut self, _clock: &RobotClock, input: &Self::Input<'i>, output: &mut Self::Output<'o>,)
    -> CuResult<()>
    {

    }
}

impl CuSrcTask for Panner {
    type Output<'m> = output_msg!(CameraPanningPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        // use this method to init iox2 sub
    }

    fn process(&mut self, clock: &RobotClock, new_msg: &mut Self::Output<'_>) -> CuResult<()> {

    }
}
