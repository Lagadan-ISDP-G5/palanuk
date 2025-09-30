use libc::*;
use env_logger::Env;
use core_affinity::*;
use std::{thread::sleep, time::*};
use hcsr04_gpio_cdev::HcSr04;

/* Minimally viable code to demonstrate HC-SR04 on the Pi 5 */
const HC_SR04_ECHO: u32 = 20; // GPIO20
const HC_SR04_TRIG: u32 = 21; // GPIO21


fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let res = unsafe {
        mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE)
    };
    match res {
        0 => {
            log::info!("mlockall() returned 0");
        }
        _ => {
            log::error!("mlockall() failed, returned {}. Make sure to run as root.", res);
        }
    }

    let _ = core_affinity::set_for_current(CoreId {id: 2}); // Cores 2-3 isolated
    let thread_param = sched_param {sched_priority: 90};
    let sched_res = unsafe {
        sched_setscheduler(0, SCHED_FIFO, &thread_param)
    };
    match sched_res {
        0 => {
            log::info!("main: sched_setscheduler call returned 0");
        },
        _ => {
            log::error!("main: sched_setscheduler failed: Returned {}. Make sure to run as root.", sched_res);
        }
    }

    let mut hcsr04 = HcSr04::new(HC_SR04_TRIG, HC_SR04_ECHO)?;
    loop {
        let distance = hcsr04.dist_cm(None)?;
        println!("Distance: {:06.2}cm", distance.to_val());
        sleep(Duration::from_secs_f32(0.2));
    }
    Ok(())
}