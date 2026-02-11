extern crate cu_bincode as bincode;
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use cu_cam_pan::{CameraPanningPayload, PositionCommand};
use cu_propulsion::{PropulsionPayload, WheelDirection};
use cu_hcsr04::{HcSr04Payload};
use opencv_splitter::NsmPayload;
use dual_mtr_ctrlr::DualMtrCtrlrPayload;

/// huge foot bazooka!!!!! this used to be
/// Closed,
/// #[default]
/// Open
///
/// which doesnt do anything because Closed has value 0 and somehow somewhere that 0 is mapped back to Closed,
/// when it shouldn've been open
#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub enum LoopState {
    #[default]
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub enum WorkOrRestState {
    #[default]
    AtRest,
    AtWork
}

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub enum SteerDirection {
    #[default]
    Free,
    HardRight,
    SlightRight,
    HardLeft,
    SlightLeft
}

#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct PropulsionAdapterOutputPayload {
    pub loop_state: LoopState,
    pub propulsion_payload: PropulsionPayload,
    pub panner_payload: CameraPanningPayload,
    pub weighted_error: f32,
    pub is_e_stop_triggered: bool,
    pub distance: Option<f64>,
}

// impl From<&PropulsionAdapterOutputPayload> for DualMtrCtrlrPayload {
//     fn from(value: &PropulsionAdapterOutputPayload) -> Self {
//         Self {
//             error : value.weighted_error
//         }
//     }
// }

#[derive(Default, Debug, Clone, Copy, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct ZenohTopicsAdapterOutputPayload {
    pub loop_state: LoopState,
    pub left_enable: bool,
    pub right_enable: bool,
    pub openloop_left_speed: f32,
    pub openloop_right_speed: f32,
    pub left_direction: WheelDirection,
    pub right_direction: WheelDirection,
    pub steer_direction: SteerDirection,
    pub work_or_rest_state: WorkOrRestState,
    pub camera_position: PositionCommand,
}

pub struct PropulsionAdapter {
    e_stop_threshold_cm: f64
}

impl Freezable for PropulsionAdapter {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.e_stop_threshold_cm, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.e_stop_threshold_cm = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuTask for PropulsionAdapter {
    type Input<'m> = input_msg!('m, ZenohTopicsAdapterOutputPayload, HcSr04Payload, NsmPayload);
    type Output<'m> = output_msg!(PropulsionAdapterOutputPayload, DualMtrCtrlrPayload);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where Self: Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for PropulsionAdapter in RON")?;

        let e_stop_threshold_cm: f64 = kv
            .get("e_stop_threshold_cm")
            .expect("e_stop_threshold_cm for PropulsionAdapter not set in RON config.")
            .clone()
            .into();

        Ok(Self { e_stop_threshold_cm })
    }

    fn process(&mut self, clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let (get_zenoh, get_hcsr04, get_nsm) = input;

        // Zenoh commands are required - can't do anything without knowing the mode
        // IMPORTANT: All subscribers from ODD must have received at least something for this to not just
        // evaluate to None
        let Some(zenoh_msg) = get_zenoh.payload() else {
            return Ok(());
        };

        let loop_state = zenoh_msg.loop_state;

        // Distance sensor: require payload (cu-hcsr04 is sticky)
        let Some(hcsr04_msg) = get_hcsr04.payload() else {
            return Ok(());
        };

        let distance = hcsr04_msg.distance;
        let is_e_stop_triggered = distance
            .map_or(false, |d| d < self.e_stop_threshold_cm);

        // NSM payload: only needed for closed-loop (heading error for PID)
        // For open-loop, use 0.0; for closed-loop, require payload (opencv-splitter is sticky)
        let weighted_error = match loop_state {
            LoopState::Closed => {
                match get_nsm.payload() {
                    Some(m) => {
                        let mut res: f32 = m.heading_error; // heading_error based on center lane, opencv-splitter is sticky so this is nonzero in SS

                        if !m.vertical_line_valid && m.corner_detected {
                            res = 0.5 - m.corner_coords.0; // we only get normalized corner coords from IPC
                            // in opencv (0,0) is top left, (1,1) bottom right!!!!!
                        }
                        if !m.vertical_line_valid && !m.corner_detected {
                            // res will still be m.heading_error
                            warning!("totally absent heading error signals!!!!")
                        }
                        res
                    },
                    None => return Ok(()),
                }
            },
            LoopState::Open => 0.0,
        };

        let mut propulsion_payload = PropulsionPayload {
            left_enable: zenoh_msg.left_enable,
            right_enable: zenoh_msg.right_enable,
            left_speed: zenoh_msg.openloop_left_speed,
            right_speed: zenoh_msg.openloop_right_speed,
            left_direction: zenoh_msg.left_direction,
            right_direction: zenoh_msg.right_direction
        };

        let panner_payload = CameraPanningPayload { pos_cmd: zenoh_msg.camera_position };

        let is_at_rest = matches!(zenoh_msg.work_or_rest_state, WorkOrRestState::AtRest);

        let stop_condition = is_e_stop_triggered || is_at_rest;
        if stop_condition {
            propulsion_payload = PropulsionPayload {
                left_enable: false,
                right_enable: false,
                left_speed: 0.0,
                right_speed: 0.0,
                left_direction: WheelDirection::Stop,
                right_direction: WheelDirection::Stop,
            };
        }
        else {
            // passthrough for openloop steercmd
            match loop_state {
                LoopState::Open => {
                    match zenoh_msg.steer_direction {
                        SteerDirection::HardLeft => {
                            propulsion_payload.left_direction = WheelDirection::Reverse;
                            propulsion_payload.right_direction = WheelDirection::Forward;
                            // propulsion_payload.left_speed = 0.44;
                        },
                        SteerDirection::HardRight => {
                            propulsion_payload.left_direction = WheelDirection::Forward;
                            propulsion_payload.right_direction = WheelDirection::Reverse;
                            // propulsion_payload.right_speed = 0.44;
                        },
                        SteerDirection::Free => {
                            propulsion_payload.left_direction = zenoh_msg.left_direction;
                            propulsion_payload.right_direction = zenoh_msg.right_direction;
                        }
                        _ => ()
                    }
                }
                _ => ()
            }
        }

        let prop_adap_output_payload = PropulsionAdapterOutputPayload {
            loop_state,
            propulsion_payload,
            panner_payload,
            weighted_error,
            is_e_stop_triggered,
            distance,
        };

        output.0.set_payload(prop_adap_output_payload);
        output.1.tov = Tov::Time(clock.now());
        output.1.set_payload(DualMtrCtrlrPayload { error: weighted_error });
        output.1.metadata.set_status(format!("hdng_err: {weighted_error:.2}"));
        Ok(())
    }
}
