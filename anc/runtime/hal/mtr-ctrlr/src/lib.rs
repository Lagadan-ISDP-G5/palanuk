use cu29::prelude::*;
use core::marker::PhantomData;
use cu_pid::GenericPIDTask;
use propulsion_adapter::{PropulsionAdapterOutputPayload, LoopState};

/// This generic task is meant to be as in the modules lmtr-ctrlr and rmtr-ctrlr, or any implementation of a PID-controlled motor

pub trait MtrCtrlrPayload: CuMsgPayload + Into<f32> + From<f32> {
}

pub type Mtr<M> = GenericPIDTask<MtrCtrlr<M>>;

pub struct MtrCtrlr<M> where M: MtrCtrlrPayload {
    _marker: PhantomData<M>,
}

impl<M> Freezable for MtrCtrlr<M> where M: MtrCtrlrPayload {}

impl<M> CuTask for MtrCtrlr<M> where M: MtrCtrlrPayload {
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
        let weighted_error = input_msg.weighted_error;

        match input_msg.loop_state {
            LoopState::Closed => {
                output.set_payload(weighted_error.into());
            },
            LoopState::Open => {
                return Ok(()) // no-op
            }
        }
        Ok(())
    }
}
