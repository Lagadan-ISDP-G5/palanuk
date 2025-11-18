use std::time::Duration;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread::{JoinHandle, spawn, sleep};
use dumb_sysfs_pwm::{Pwm, Polarity};
use cu29::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// We're assuming that during operation, we won't send commands before the previous sent command has been
/// fully actuated by the servo. The SG90 is a cheap, crappy servo that easily gets confused by quick
/// changes in the PWM duty cycle. No point in overengineering the code (it already is) just to handle
/// HW race conditions arising from cheap HW.

const PERIOD_NS: u32 = 20000000; /// Period in ns for 50Hz
const DUTY_CYCLE_POS_FRONT: f32 = 0.075; /// 1.5ms out of 20ms
const DUTY_CYCLE_POS_LEFT: f32 = 0.1; /// 1.0ms out of 20ms
const DUTY_CYCLE_POS_RIGHT: f32 = 0.05; /// 2.0ms out of 20ms
const IPOLATE_DIV: f32 = 1000.0;

// Not used here, the assignment is final but it should be passed in the RON instead of being hardcoded
const _SG90_POS_CMD: u32 = 12;

/// this payload has no HW feedback
#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct CameraPanningPayload {
    pub pos_cmd: PositionCommand,
    // active_cfg: CameraPanningPinAssignments
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub enum PositionCommand {
    #[default]
    Front,
    Left,
    Right
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct CameraPanningPinAssignments {
    sg90_pos_cmd_pin: u32,
}

pub struct CameraPanningControllerInstances {
    sg90_pos_cmd: Pwm
}

pub struct CameraPanning {
    task_running: Arc<AtomicBool>,
    recvd_pos_cmd: Arc<Mutex<PositionCommand>>,
    pin_controller_instances: Arc<CameraPanningControllerInstances>,
    ipolate_thread_hdl: Option<JoinHandle<()>>,
    pin_assignments: CameraPanningPinAssignments,
}

impl Freezable for CameraPanning {
    fn freeze<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        Encode::encode(&self.recvd_pos_cmd, encoder)?;
        Encode::encode(&self.pin_assignments, encoder)?;
        Ok(())
    }

    fn thaw<D: bincode::de::Decoder>(&mut self, decoder: &mut D) -> Result<(), bincode::error::DecodeError> {
        self.recvd_pos_cmd = Decode::decode(decoder)?;
        Ok(())
    }
}

impl CuSinkTask for CameraPanning {
    type Input<'m> = input_msg!(CameraPanningPayload);
    fn new(config: Option<&ComponentConfig>) -> Result<Self, CuError>
    where Self: Sized
    {
        let ComponentConfig(kv) =
            config.ok_or("No ComponentConfig specified for GPIO in RON")?;

        let sg90_pos_cmd_pin_offset: u32 = kv
            .get("sg90_pos_cmd_pin")
            .expect("l298n_en_a for Propulsion not set in RON config")
            .clone()
            .into();

        let pin_assignments = CameraPanningPinAssignments {
            sg90_pos_cmd_pin: sg90_pos_cmd_pin_offset
        };

        // #[cfg(hardware)]
        let sg90_pos_cmd_instance = Pwm::new(0, sg90_pos_cmd_pin_offset).unwrap();
        let pin_controller_instances = CameraPanningControllerInstances {
            sg90_pos_cmd: sg90_pos_cmd_instance
        };

        Ok(Self {
            task_running: Arc::new(AtomicBool::new(true)),
            recvd_pos_cmd: Arc::new(Mutex::new(PositionCommand::default())),
            ipolate_thread_hdl: None,
            pin_controller_instances: Arc::new(pin_controller_instances),
            pin_assignments: pin_assignments,
        })
    }

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        let task_running = Arc::clone(&self.task_running);
        let pos_cmd = Arc::clone(&self.recvd_pos_cmd);
        let controller = Arc::clone(&self.pin_controller_instances);
        let mut pos_cmd_copy: PositionCommand = PositionCommand::Front;

        let ipolate_thread_hdl = spawn(move || {
            // make sure PWM params are initialized
            _ = controller.sg90_pos_cmd.export(); // export first
            _ = controller.sg90_pos_cmd.set_period_ns(PERIOD_NS);
            _ = controller.sg90_pos_cmd.set_polarity(Polarity::Normal);

            // check if controller is enabled yet
            if !controller.sg90_pos_cmd.get_enabled().unwrap() {
                controller.sg90_pos_cmd.enable(true).unwrap();
                // should probably add a reset routine here, with a helper function to make sure the servo
                // resets to the front position
            }

            // Initialize at middle position
            _ = controller.sg90_pos_cmd.set_duty_cycle(0.075);
            sleep(Duration::from_millis(1750));

            while task_running.load(Ordering::Relaxed) {
                let rd_guard = match pos_cmd.try_lock() {
                    Ok(val) => Some(val),
                    Err(_) => None
                };

                match rd_guard {
                    Some(cmd) => {
                        pos_cmd_copy = (*cmd).clone();
                    },
                    _ => {} // Do nothing if process() method in main thread happens to write to pos_cmd
                }

                // FIXME: remove wasteful casts
                let target_duty_cycle = match pos_cmd_copy {
                    PositionCommand::Front => {
                        (DUTY_CYCLE_POS_FRONT * IPOLATE_DIV) as u32
                    },
                    PositionCommand::Left => {
                        (DUTY_CYCLE_POS_LEFT * IPOLATE_DIV) as u32
                    },
                    PositionCommand::Right => {
                        (DUTY_CYCLE_POS_RIGHT * IPOLATE_DIV) as u32
                    }
                };
                // should probably make this task stateful, i.e., remembers what position it's in so
                // that our for loop starts from the target_duty_cycle it was prior
                let start = (DUTY_CYCLE_POS_FRONT * IPOLATE_DIV) as u32;

                if target_duty_cycle == start {
                    _ = controller.sg90_pos_cmd.set_duty_cycle(DUTY_CYCLE_POS_FRONT);
                    sleep(Duration::from_millis(1750));
                }
                else {
                    for duty_cycle in (start..=target_duty_cycle).step_by(1) {
                        _ = controller.sg90_pos_cmd.set_duty_cycle(duty_cycle as f32 / IPOLATE_DIV);
                        sleep(Duration::from_millis(10));
                    }

                    sleep(Duration::from_millis(1750));

                    for duty_cycle in (start..=target_duty_cycle).rev().step_by(1) {
                        _ = controller.sg90_pos_cmd.set_duty_cycle(duty_cycle as f32 / IPOLATE_DIV);
                        sleep(Duration::from_millis(10));
                    }
                }

            }

            // Cleanup
            // Return back to middle position
            sleep(Duration::from_millis(1750));
            _ = controller.sg90_pos_cmd.set_duty_cycle(0.075);
            sleep(Duration::from_millis(1750));

            _ = controller.sg90_pos_cmd.enable(false);
            _ = controller.sg90_pos_cmd.set_duty_cycle(0.0);
            _ = controller.sg90_pos_cmd.unexport();
        });

        self.ipolate_thread_hdl = Some(ipolate_thread_hdl);
        Ok(())
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>) -> Result<(), CuError> {
        let pos_cmd = Arc::clone(&mut self.recvd_pos_cmd);
        let payload = input.payload().unwrap();
        let rw_guard = match pos_cmd.try_lock() {
            Ok(val) => Some(val),
            Err(_) => None,
        };
        match rw_guard {
            Some(mut pos_cmd) => {
                *pos_cmd = payload.pos_cmd
            },
            None => ()
        }
        Ok(())
    }

    fn stop(&mut self, _clock: &RobotClock) -> CuResult<()> {
        self.task_running.store(false, Ordering::Relaxed);
        let hdl = self.ipolate_thread_hdl.take();
        match hdl {
            Some(hdl) => {
                hdl.join().expect("CameraPanning PWM duty cycle interpolation thread panicked upon stop command issued")
            },
            None => ()
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
