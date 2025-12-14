use cu29::prelude::*;
use cu29_export::run_cli;

gen_cumsgs!("rtimecfg.ron");

fn main() {
    run_cli::<CuMsgs>().expect("Failed to run the export CLI");
}
