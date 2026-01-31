use cu29::prelude::*;
use cu29_export::run_cli;
use ec_pub::*;

gen_cumsgs!("taskdag.ron");

fn main() {
    run_cli::<CuMsgs>().expect("Failed to run the export CLI");
}
