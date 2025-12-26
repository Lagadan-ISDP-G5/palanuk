use cu29::prelude::*;
use core::marker::PhantomData;
use bincode::{Decode, Encode};
use propulsion_adapter::PropulsionAdapterOutputPayload;

// pub struct MtrPayload<M> where M: CuMsgPayload + Into<f32> {
//     _marker: PhantomData<M>,
//     pub weighted_error: f32
// }

pub trait MtrWeightedErrorPayload: CuMsgPayload + Into<f32> {}

pub struct MtrCtrlr<M> where M: MtrWeightedErrorPayload {
    _marker: PhantomData<M>,
}

impl<M> Freezable for MtrCtrlr<M> where M: MtrWeightedErrorPayload {}

impl<M> CuTask for MtrCtrlr<M> where M: MtrWeightedErrorPayload {
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload);
    type Output<'m> = output_msg!(M);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
        where
            Self: Sized, {
        Ok(Self { _marker: PhantomData })
    }

    fn process<'i, 'o>(
            &mut self,
            _clock: &RobotClock,
            input: &Self::Input<'i>,
            output: &mut Self::Output<'o>,
        ) -> CuResult<()> {
        let input_msg = input.payload().map_or(Err(CuError::from(format!("none payload cip"))), |msg| {Ok(msg)})?;

        // let weighted_error = input_msg.
    }
}
