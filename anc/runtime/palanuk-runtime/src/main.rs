use cu29::{clock, prelude::*};
use cu29_helpers::basic_copper_setup;
use std::fs;
use std::path::{Path, PathBuf};

#[copper_runtime(config = "rtimecfg.ron")]
struct Palanuk {}

use cu29::prelude::*;
use iceoryx2::prelude::*;
use cu_propulsion::{PropulsionPayload, WheelDirection};
use cu_cam_pan::{CameraPanningPayload};
use cu_hcsr04::{HcSr04Payload};
use cu_powermon::*;

pub struct Jogger {}
// pub struct Panner {}

impl Freezable for Jogger {}
// impl Freezable for Panner {}

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

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>, output: &mut Self::Output<'_>,)
    -> CuResult<()>
    {
        let hcsr04_msg = *input;
        let mut dist: f64 = 0.0;

        match hcsr04_msg.payload() {
            Some(payload) => dist = payload.distance,
            _ => {}
        }

        if dist < 10.0 {
            output.set_payload(PropulsionPayload {
                left_enable: false,
                right_enable: false,
                left_direction: WheelDirection::Forward,
                right_direction: WheelDirection::Forward,
                left_speed: 0.0,
                right_speed: 0.0
            });
        }
        Ok(())
    }
}

// impl CuSrcTask for Panner {
//     type Output<'m> = output_msg!(CameraPanningPayload);

//     fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
//     where Self: Sized
//     {
//         Ok(Self {})
//     }

//     fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
//         // use this method to init iox2 sub
//     }

//     fn process(&mut self, clock: &RobotClock, new_msg: &mut Self::Output<'_>) -> CuResult<()> {

//     }
// }


fn main() {
    let logger_path = "logs/palanuk.copper";
    if let Some(parent) = Path::new(logger_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create logs directory");
        }
    }

    let copper_ctx = basic_copper_setup(&PathBuf::from(logger_path), true).expect("Failed to setup logger.");
    debug!("Logger created at {}.", path = logger_path);
    debug!("Creating application... ");

    let clock = copper_ctx.clock;

    let mut application = Palanuk::new(
        clock.clone(),
        copper_ctx.clone()
    ).expect("Failed to create runtime.");


    debug!("Running... starting clock: {}.", clock.now());
    application.run().expect("Failed to run application.");
    debug!("End of app: final clock: {}.", clock.now());
}
