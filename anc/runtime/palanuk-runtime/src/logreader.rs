#![allow(unused_imports)]
#![allow(unused_import_braces)]

use cu29::prelude::*;
use cu29_export::run_cli;
use anc_pub::{ObstacleDetected, Distance};
use ec_pub::{PowerMwatts, LoadCurrentMamps, BusVoltageMvolts, ShuntVoltageMvolts};
use zsrc_merger::{OddOpenLoopSpeed, OddLoopMode, OddOpenLoopDriveState, OddOpenLoopForcepan};

gen_cumsgs!("taskdag.ron");

fn main() {
    run_cli::<CuMsgs>().expect("Failed to run the export CLI");
}
