use std::{str::FromStr, sync::{Arc, atomic::{AtomicBool, AtomicU8, Ordering}}};
use std::thread::{JoinHandle, spawn, Builder};
use std::time::{Duration, Instant};
use libc::*;
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

// Just to test
#[inline(always)]
pub fn plnk_busy_wait_for(duration: Duration) {
    let sum = Instant::now().checked_add(duration);
    match sum {
        Some(sum) => plnk_busy_wait_until(sum),
        None => ()
    }
}
#[inline(always)]
pub fn plnk_busy_wait_until(time: Instant) {
    while Instant::now() < time {
        core::hint::spin_loop();
    }
}

/// this payload has no HW feedback
#[derive(Debug, Clone, Copy, Default, Encode, Decode, PartialEq, Serialize, Deserialize)]
pub struct CameraPanningPayload {
    pub pos_cmd: PositionCommand,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub enum PositionCommand {
    #[default]
    Front,
    Left,
    Right
}

impl PositionCommand {
    #[inline(always)]
    fn to_u8(self) -> u8 {
        match self {
            PositionCommand::Front => 0,
            PositionCommand::Left  => 1,
            PositionCommand::Right => 2,
        }
    }

    #[inline(always)]
    fn from_u8(x: u8) -> PositionCommand {
        match x {
            0 => PositionCommand::Front,
            1 => PositionCommand::Left,
            2 => PositionCommand::Right,
            _ => PositionCommand::Front,  // fallback / sanitize
        }
    }
}

pub struct CameraPanningControllerInstances {
    sg90_pos_cmd: Pwm
}

pub struct CameraPanning {
    task_running: Arc<AtomicBool>,
    recvd_pos_cmd: Arc<AtomicU8>,
    pin_controller_instances: Arc<CameraPanningControllerInstances>,
    ipolate_thread_hdl: Option<JoinHandle<Result<(), cu29::CuError>>>,
}

impl Freezable for CameraPanning {}

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

        let sg90_pos_cmd_instance = Pwm::new(0, sg90_pos_cmd_pin_offset).unwrap();
        let pin_controller_instances = CameraPanningControllerInstances {
            sg90_pos_cmd: sg90_pos_cmd_instance
        };

        Ok(Self {
            task_running: Arc::new(AtomicBool::new(true)),
            recvd_pos_cmd: Arc::new(AtomicU8::new(PositionCommand::to_u8(PositionCommand::default()))),
            ipolate_thread_hdl: None,
            pin_controller_instances: Arc::new(pin_controller_instances),
        })
    }

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        let task_running = Arc::clone(&self.task_running);
        let pos_cmd = Arc::clone(&self.recvd_pos_cmd);
        let controller = Arc::clone(&self.pin_controller_instances);

        let ipolate_thread_hdl = Builder::new()
            .name(String::from_str("cu-cam-pan-ipolate-thread").unwrap())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || -> CuResult<()> {
            let thread_param = sched_param {sched_priority: 70};
            let sched_res = unsafe {
                sched_setscheduler(0, SCHED_RR, &thread_param)
            };
            match sched_res {
                0 => {
                    info!("cu-cam-pan ipolate thread: sched_setscheduler call returned 0");
                },
                _ => { // Refer here: https://man7.org/linux/man-pages/man2/sched_setscheduler.2.html
                    return Err(CuError::from("cu-cam-pan ipolate thread: sched_setscheduler call returned -1. sched_setscheduler failed."));
                }
            }

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
            plnk_busy_wait_for(Duration::from_millis(1750));

            while task_running.load(Ordering::Relaxed) {
                let pos_cmd_copy = PositionCommand::from_u8(pos_cmd.load(Ordering::Acquire));
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
                    plnk_busy_wait_for(Duration::from_millis(1750));
                }
                else {
                    if start < target_duty_cycle {
                        for duty_cycle in (start..=target_duty_cycle).step_by(1) {
                            _ = controller.sg90_pos_cmd.set_duty_cycle(duty_cycle as f32 / IPOLATE_DIV);
                            plnk_busy_wait_for(Duration::from_millis(10));
                        }

                        plnk_busy_wait_for(Duration::from_millis(1750));
                        for duty_cycle in (start..=target_duty_cycle).rev().step_by(1) {
                            _ = controller.sg90_pos_cmd.set_duty_cycle(duty_cycle as f32 / IPOLATE_DIV);
                            plnk_busy_wait_for(Duration::from_millis(10));
                        }
                    }
                    else {
                        for duty_cycle in (target_duty_cycle..=start).step_by(1) {
                            _ = controller.sg90_pos_cmd.set_duty_cycle(duty_cycle as f32 / IPOLATE_DIV);
                            plnk_busy_wait_for(Duration::from_millis(10));
                        }

                        plnk_busy_wait_for(Duration::from_millis(1750));
                        for duty_cycle in (target_duty_cycle..=start).rev().step_by(1) {
                            _ = controller.sg90_pos_cmd.set_duty_cycle(duty_cycle as f32 / IPOLATE_DIV);
                            plnk_busy_wait_for(Duration::from_millis(10));
                        }
                    }
                }
            }

            // Cleanup
            // Return back to middle position
            _ = controller.sg90_pos_cmd.set_duty_cycle(0.075);
            plnk_busy_wait_for(Duration::from_millis(1750));

            _ = controller.sg90_pos_cmd.enable(false);
            _ = controller.sg90_pos_cmd.set_duty_cycle(0.0);
            _ = controller.sg90_pos_cmd.unexport();
            Ok(())
        });

        self.ipolate_thread_hdl = Some(ipolate_thread_hdl);
        Ok(())
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>) -> Result<(), CuError> {
        let pos_cmd = Arc::clone(&mut self.recvd_pos_cmd);
        let payload = input.payload();
        match payload {
            Some(nonempty_payload) => {
                pos_cmd.store(nonempty_payload.pos_cmd.to_u8(), Ordering::Release);
            },
            None => () // no-op
        }
        Ok(())
    }

    fn stop(&mut self, _clock: &RobotClock) -> CuResult<()> {
        self.task_running.store(false, Ordering::Relaxed);
        let hdl = self.ipolate_thread_hdl.take();
        match hdl {
            Some(hdl) => {
                let ret = hdl.join().expect("CameraPanning PWM duty cycle interpolation thread panicked upon stop command issued");
                match ret {
                    Ok(_) => (),
                    Err(_) => return Err(CuError::from("Failed to stop cu-cam-pan"))
                }
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
