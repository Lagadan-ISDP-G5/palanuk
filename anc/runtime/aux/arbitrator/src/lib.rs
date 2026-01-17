use std::pin::Pin;

/// This task consolidates the arbitration of permissives/interlocks from different inputs
/// This is where it decides that an e-stop condition is correct, and also ultimately decides
/// if the loop mode can change. This task is stateful; it has feedback values for e-stop trigger
/// and loop mode.

use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use propulsion_adapter::{LoopState, PropulsionAdapterOutputPayload};
use cu_propulsion::PropulsionPayload;
use cu_pid::PIDControlOutputPayload;
use herald::HeraldNewsPayload;

pub struct Arbitrator {
    e_stop_trig_fdbk: bool,
    loop_mode_fdbk: LoopState,
}

impl Default for Arbitrator {
    fn default() -> Self {
        Self {
            e_stop_trig_fdbk: false,
            loop_mode_fdbk: LoopState::Closed
        }
    }
}

impl Freezable for Arbitrator {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.e_stop_trig_fdbk, encoder)?;
        Encode::encode(&self.loop_mode_fdbk, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.e_stop_trig_fdbk = Decode::decode(decoder)?;
        self.loop_mode_fdbk = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuTask for Arbitrator {
    type Input<'m> = input_msg!('m, PropulsionAdapterOutputPayload, PIDControlOutputPayload, PIDControlOutputPayload);
    type Output<'m> = output_msg!((PropulsionPayload, PropulsionPayload, HeraldNewsPayload));

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self::default())
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>)
    -> CuResult<()>
    {
        let (prop_adap, lmtr_pid, rmtr_pid) = *input;

        if let (Some(prop_adap_pload), Some(lmtr_pid_pload), Some(rmtr_pid_pload)) = (prop_adap.payload(), lmtr_pid.payload(), rmtr_pid.payload()) {

            let is_e_stop_triggered = prop_adap_pload.is_e_stop_triggered;
            let loop_state = prop_adap_pload.loop_state;

            match is_e_stop_triggered {
                true => (),
                false => ()
            }

            match loop_state {
                LoopState::Closed => (),
                LoopState::Open => ()
            }

            let lmtr_pid_output = lmtr_pid_pload.output;
            let rmtr_pid_output = rmtr_pid_pload.output;

            // TODO: link these PID outputs into values in PropulsionPayload

        }

        Ok(())
    }
}
