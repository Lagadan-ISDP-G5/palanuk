use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::fs;
use std::path::{Path, PathBuf};

use cu_propulsion::{PropulsionPayload, WheelDirection};
use cu_cam_pan::{CameraPanningPayload, PositionCommand};
use cu_hcsr04::{HcSr04Payload};
use cu_powermon::{Ina219Payload};
use ec_pub::*;
use zsrc_merger::*;
use opencv_iox2::*;
use propulsion_adapter::*;
use panner_adapter::*;
use dual_mtr_ctrlr::*;
use arbitrator::*;
use anc_pub::*;
use propulsion_cleaver::*;
use dual_mtr_ctrlr::*;
use cu_pid::*;

use core_affinity::*;
use libc::*;

pub mod odd_subs {
    use cu_zenoh_src::ZSrc;
    use zsrc_merger::{OddOpenLoopSpeed, OddOpenLoopStop, OddLoopMode, OddOpenLoopDriveState, OddOpenLoopForcepan};

    pub type OddOpenLoopSpeedSrc      = ZSrc<zsrc_merger::OddOpenLoopSpeed>;
    pub type OddOpenLoopStopSrc       = ZSrc<zsrc_merger::OddOpenLoopStop>;
    pub type OddOpenLoopModeSrc       = ZSrc<zsrc_merger::OddLoopMode>;
    pub type OddOpenLoopDriveStateSrc = ZSrc<zsrc_merger::OddOpenLoopDriveState>;
    pub type OddOpenLoopForcepanSrc   = ZSrc<zsrc_merger::OddOpenLoopForcepan>;
}

pub mod anc_pubs {
    use cu_zenoh_sink::ZSink;
    use anc_pub::{ObstacleDetected, Distance};

    pub type ObstacleDetectedSink = ZSink<anc_pub::ObstacleDetected>;
    pub type DistanceSink         = ZSink<anc_pub::Distance>;
}


pub mod ec_5vrail_pubs {
    use cu_zenoh_sink::ZSink;
    use ec_pub::{PowerMwatts, LoadCurrentMamps, BusVoltageMvolts, ShuntVoltageMvolts};

    pub type PowerMwattsSink        = ZSink<ec_pub::PowerMwatts>;
    pub type LoadCurrentMampsSink   = ZSink<ec_pub::LoadCurrentMamps>;
    pub type BusVoltageMvoltsSink   = ZSink<ec_pub::BusVoltageMvolts>;
    pub type ShuntVoltageMvoltsSink = ZSink<ec_pub::ShuntVoltageMvolts>;
}

// pub mod ec_lmtr_pubs {
//     use cu_zenoh_sink::ZSink;
//     use ec_pub::{PowerMwatts, LoadCurrentMamps, BusVoltageMvolts, ShuntVoltageMvolts};

//     pub type PowerMwattsSink        = ZSink<ec_pub::PowerMwatts>;
//     pub type LoadCurrentMampsSink   = ZSink<ec_pub::LoadCurrentMamps>;
//     pub type BusVoltageMvoltsSink   = ZSink<ec_pub::BusVoltageMvolts>;
//     pub type ShuntVoltageMvoltsSink = ZSink<ec_pub::ShuntVoltageMvolts>;
// }


// pub mod ec_rmtr_pubs {
//     use cu_zenoh_sink::ZSink;
//     use ec_pub::{PowerMwatts, LoadCurrentMamps, BusVoltageMvolts, ShuntVoltageMvolts};

//     pub type PowerMwattsSink        = ZSink<ec_pub::PowerMwatts>;
//     pub type LoadCurrentMampsSink   = ZSink<ec_pub::LoadCurrentMamps>;
//     pub type BusVoltageMvoltsSink   = ZSink<ec_pub::BusVoltageMvolts>;
//     pub type ShuntVoltageMvoltsSink = ZSink<ec_pub::ShuntVoltageMvolts>;
// }

#[copper_runtime(config = "taskdag.ron", sim_mode = false)]
struct Palanuk {}

#[allow(clippy::identity_op)]
const SLAB_SIZE: Option<usize> = Some(1 * 1024 * 1024 * 1024);

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

    #[cfg(target_os = "linux")]
    let _ = core_affinity::set_for_current(CoreId {id: 2}); // Cores 2-3 isolated
    #[cfg(target_os = "linux")]
    let thread_param = sched_param {sched_priority: 92};
    let sched_res = unsafe {
        #[cfg(target_os = "linux")]
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

    application.run().expect("Failed to run application."); // blocks indefinitely

    debug!("End of app: final clock: {}.", clock.now());
}
