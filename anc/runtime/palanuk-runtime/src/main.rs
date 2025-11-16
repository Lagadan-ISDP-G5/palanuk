pub mod tasks;

use cu29::{clock, prelude::*};
use std::fs;
use std::path::{Path, PathBuf};

#[copper_runtime(config = "rtimecfg.ron")]
struct Palanuk {}

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
