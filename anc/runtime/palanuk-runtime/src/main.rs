use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use cu_propulsion::{PropulsionPayload, WheelDirection};
use cu_cam_pan::{CameraPanningPayload, PositionCommand};
use cu_hcsr04::HcSr04Payload;
use cu_powermon::{CuIna219, Ina219Payload};
use ctrlc::*;
use std::sync::mpsc::channel;
use core_affinity::*;
use libc::*;
// use iceoryx2::prelude::*;

#[copper_runtime(config = "rtimecfg.ron", sim_mode = false)]
struct Palanuk {}

#[allow(clippy::identity_op)]
const SLAB_SIZE: Option<usize> = Some(1 * 1024 * 1024 * 1024);

pub struct Jogger {}
pub struct Panner {}
pub struct Dummy {}

impl Freezable for Jogger {}
impl Freezable for Panner {}
impl Freezable for Dummy {}

impl CuSinkTask for Dummy {
    type Input<'m> = input_msg!('m, Ina219Payload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    fn process(
            &mut self,
            _clock: &RobotClock,
            input: &Self::Input<'_>,
        ) -> CuResult<()> {
        let dummy = input.payload();

        Ok(())
    }

}

impl CuTask for Jogger {
    type Input<'m> = input_msg!('m, HcSr04Payload);
    type Output<'m> = output_msg!(PropulsionPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    // fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
    //     // use this method to init iox2 sub
    //     Ok(())
    // }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let hcsr04_msg = input;
        let mut dist: f64 = 0.0;

        match hcsr04_msg.payload() {
            Some(payload) => dist = payload.distance,
            _ => {}
        }

        if dist < 10.0 {
            output.set_payload(PropulsionPayload {
                left_enable: false,
                right_enable: false,
                left_direction: WheelDirection::Stop,
                right_direction: WheelDirection::Stop,
                left_speed: 0.0,
                right_speed: 0.0
            });

            output.metadata.set_status(format!("Stopped. Obstacle detected."));
        }
        else {
            output.set_payload(PropulsionPayload {
                left_enable: true,
                right_enable: true,
                left_direction: WheelDirection::Forward,
                right_direction: WheelDirection::Forward,
                left_speed: 1.0,
                right_speed: 0.01
            });

            output.metadata.set_status(format!("Moving..."));
        }
        Ok(())
    }
}

impl CuTask for Panner {
    type Input<'m> = input_msg!('m, HcSr04Payload);
    type Output<'m> = output_msg!(CameraPanningPayload);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where Self: Sized
    {
        Ok(Self {})
    }

    // fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
    //     // use this method to init iox2 sub
    // }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let hcsr04_msg = input;
        let mut dist: f64 = 0.0;

        match hcsr04_msg.payload() {
            Some(payload) => dist = payload.distance,
            _ => {}
        }

        if dist < 10.0 {
            output.set_payload(CameraPanningPayload {
                pos_cmd: PositionCommand::Left
            });

            output.metadata.set_status(format!("Camera Panning Left."));
        }
        else {
            output.set_payload(CameraPanningPayload {
                pos_cmd: PositionCommand::Front
            });

            output.metadata.set_status(format!("Camera Panning Front"));
        }
        Ok(())
    }
}


fn main() {
    let res = unsafe {
        mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE)
    };
    match res {
        0 => {
            info!("mlockall() returned 0");
        }
        _ => {
            error!("mlockall() failed, returned {}. Make sure to run as root.", res);
        }
    }

    let _ = core_affinity::set_for_current(CoreId {id: 2}); // Cores 2-3 isolated
    let thread_param = sched_param {sched_priority: 90};
    let sched_res = unsafe {
        sched_setscheduler(0, SCHED_RR, &thread_param)
    };
    match sched_res {
        0 => {
            info!("main: sched_setscheduler call returned 0");
        },
        _ => {
            error!("main: sched_setscheduler failed: Returned {}. Make sure to run as root.", sched_res);
        }
    }

    let logger_path = "logs/palanuk.copper";
    if let Some(parent) = Path::new(logger_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create logs directory");
        }
    }

    let copper_ctx = basic_copper_setup(
        &PathBuf::from(logger_path),
        SLAB_SIZE,
        false,
        None
    )
    .expect("Failed to setup logger.");
    debug!("Logger created at {}.", path = logger_path);
    debug!("Creating application... ");

    let clock = copper_ctx.clock;

    let mut application = Palanuk::new(
        clock.clone(),
        copper_ctx.unified_logger.clone(),
        None
    ).expect("Failed to create runtime.");

    // let running = Arc::new(AtomicBool::new(true));
    // let running_clone = Arc::clone(&running);
    // let (tx, rx) = channel::<AtomicBool>();

    // ctrlc::set_handler(move || {
    //     running_clone.store(false, Ordering::SeqCst);
    // })
    // .expect("Error setting Ctrl-C handler");

    // debug!("Running... starting clock: {}.", clock.now());
    // while running.load(Ordering::Relaxed) {
    //     _ = application.run_one_iteration();
    // }

    application.run().expect("Failed to run application."); // blocks indefinitely
    debug!("End of app: final clock: {}.", clock.now());
}
