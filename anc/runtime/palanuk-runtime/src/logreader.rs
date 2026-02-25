use cu29::prelude::*;
use cu29_export::*;

gen_cumsgs!("taskdag.ron");

fn main() {
    run_cli::<CuStampedDataSet>().expect("logreader failed");
}
