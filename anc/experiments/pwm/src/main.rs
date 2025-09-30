use std::error::Error;
use std::time::Duration;
use rpi_pal::pwm::{Channel, Polarity, Pwm};
use gpio_cdev::{Chip, LineHandle, LineRequestFlags};
use libc::*;
use env_logger::Env;
use core_affinity::*;

const PERIOD_MILLISEC: u64 = 50;
// const DUTY_CYCLE: f64 = 0.5;
const L298N_IN_3: u32 = 26; // GPIO26
const L298N_IN_4: u32 = 19; // GPIO19
const L298N_EN_B: Channel = Channel::Pwm1; // GPIO13

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

    let mut chip = Chip::new("/dev/gpiochip4")?; // For some reason it's /dev/gpiochip4 on the Pi 5
    let in_3_hndl = chip
        .get_line(L298N_IN_3)?
        .request(LineRequestFlags::OUTPUT, 0, "l298n-in-3")?;

    let in_4_hndl = chip
        .get_line(L298N_IN_4)?
        .request(LineRequestFlags::OUTPUT, 0, "l298n-in-4")?;

    // Move forward
    set_direction_forward(&in_3_hndl, &in_4_hndl)?;
    std::thread::sleep(Duration::from_secs_f32(1.0));

    let en_b_hndl = Pwm::with_pwmchip(0, L298N_EN_B as u8)?; // GPIO13 if Pwm1
    en_b_hndl.set_period(Duration::from_millis(PERIOD_MILLISEC))?;
    en_b_hndl.set_polarity(Polarity::Normal)?;
    en_b_hndl.enable()?;
    std::thread::sleep(Duration::from_secs_f32(1.0));

    for _ in 1..=1 {
        log::info!("Duty cycle 100%");
        en_b_hndl.set_duty_cycle(1.0)?;
        std::thread::sleep(Duration::from_secs_f32(4.0));

        log::info!("Duty cycle 80%");
        en_b_hndl.set_duty_cycle(0.8)?;
        std::thread::sleep(Duration::from_secs_f32(4.0));

        log::info!("Duty cycle 75%");
        en_b_hndl.set_duty_cycle(0.75)?;
        std::thread::sleep(Duration::from_secs_f32(4.0));

        log::info!("Duty cycle 60%");
        en_b_hndl.set_duty_cycle(0.6)?;
        std::thread::sleep(Duration::from_secs_f32(4.0));

        // log::info!("Duty cycle 50%");
        // en_b_hndl.set_duty_cycle(0.50)?;
        // std::thread::sleep(Duration::from_secs_f32(4.0));
    }

    Ok(())
    // When the pwm variable goes out of scope, the PWM channel is automatically disabled.
    // You can manually disable the channel by calling the Pwm::disable() method.
}

fn set_direction_forward(in_3_hndl: &LineHandle, in_4_hndl: &LineHandle) -> Result<(), gpio_cdev::Error> {
    in_3_hndl.set_value(1)?;
    in_4_hndl.set_value(0)?;
    Ok(())
}