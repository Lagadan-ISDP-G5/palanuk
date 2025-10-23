use std::error::Error;
use std::time::Duration;
use env_logger::Env;
use libc::*;
use rpi_pal::pwm::{Channel, Polarity, Pwm};
use core_affinity::*;

const CTRL_PWM: Channel = Channel::Pwm0; // GPIO12, PWM0

fn main() -> Result<(), Box<dyn Error>> {
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

    std::thread::sleep(Duration::from_millis(1000));

    let ctrl_pwm_hndl = Pwm::with_pwmchip(0, CTRL_PWM as u8)?; // GPIO12 if Pwm0
    ctrl_pwm_hndl.set_frequency(50.0, 0.0)?;
    ctrl_pwm_hndl.set_polarity(Polarity::Normal)?;
    ctrl_pwm_hndl.enable()?;

    loop {
        for duty in (500..=1000).step_by(2) {
            ctrl_pwm_hndl.set_duty_cycle(duty as f64 / 10000.0)?;
            std::thread::sleep(Duration::from_millis(10));
        }
        // std::thread::sleep(Duration::from_millis(1200));

        for duty in (500..=1000).rev().step_by(2) {
            ctrl_pwm_hndl.set_duty_cycle(duty as f64 / 10000.0)?;
            std::thread::sleep(Duration::from_millis(10));
        }

        std::thread::sleep(Duration::from_millis(300));
    }

    // interpolate from 0 deg to 180 deg
    // 50 -> 0 deg
    // 75 -> 90 deg
    // 100 -> 180 deg
    // for duty in (50..=75).step_by(1) {
    //     ctrl_pwm_hndl.set_duty_cycle(duty as f64 / 1000.0)?;
    //     std::thread::sleep(Duration::from_millis(20));
    // }

    // std::thread::sleep(Duration::from_millis(300));

    // for duty in (50..=75).rev().step_by(1) {
    //     ctrl_pwm_hndl.set_duty_cycle(duty as f64 / 1000.0)?;
    //     std::thread::sleep(Duration::from_millis(20));
    // }

    // std::thread::sleep(Duration::from_millis(300));

    // for duty in (75..=100).step_by(1) {
    //     ctrl_pwm_hndl.set_duty_cycle(duty as f64 / 1000.0)?;
    //     std::thread::sleep(Duration::from_millis(20));
    // }

    // std::thread::sleep(Duration::from_millis(300));

    // for duty in (75..=100).rev().step_by(1) {
    //     ctrl_pwm_hndl.set_duty_cycle(duty as f64 / 1000.0)?;
    //     std::thread::sleep(Duration::from_millis(20));
    // }

    Ok(())
}
