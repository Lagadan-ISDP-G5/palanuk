use cu29::prelude::*;
use cu29_export::run_cli;
use ec_pub::{PowerMwatts, LoadCurrentMamps, BusVoltageMvolts, ShuntVoltageMvolts};
use zsrc_merger::{OddOpenLoopSpeed, OddOpenLoopStop, OddLoopMode, OddOpenLoopDriveState, OddOpenLoopForcepan};

gen_cumsgs!("taskdag.ron");

fn main() {
    run_cli::<CuMsgs>().expect("Failed to run the export CLI");
}
