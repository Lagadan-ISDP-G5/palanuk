use gpio_cdev::{Chip, Error, LineHandle, LineRequestFlags};
use libc::*;
use env_logger::Env;
use core_affinity::*;
use std::time::*;

/* Minimally viable code to demonstrate HC-SR04 on the Pi 5 */

#[derive(PartialEq)]
pub enum TrigState {
    NotTriggered,
    Triggered
}

#[derive(PartialEq)]
pub enum EchoState {
    NotTransmitted, /* Only correct in Init condition, this should not be valid in steady-state */
    Transmitted,
    Reflected,
}

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
            log::error!("mlockall() failed, returned {}", res);
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
            log::error!("main: sched_setscheduler failed: Returned {}", sched_res);
        }
    }

    let mut chip = Chip::new("/dev/gpiochip4")?; // For some reason it's /dev/gpiochip4 on the Pi 5
    let trig_hndl = chip
        .get_line(21)? // Pi GPIO21
        .request(LineRequestFlags::OUTPUT, 0, "hc-sr04-trigger")?;

    let echo_hndl = chip
        .get_line(20)? // Pi GPIO20
        .request(LineRequestFlags::INPUT, 0, "hc-sr04-echo")?;

    let mut counter: u64 = 0;
    let cycle = Duration::from_micros(10); // 10us resolution, monotonic clock so we'll definitely hold trig HIGH for at least 10us

    // All of this data should really be stored in a struct, exposed by a library
    let mut next_time = Instant::now() + cycle;
    let mut trig_state: TrigState = TrigState::NotTriggered;
    let mut echo_state: EchoState = EchoState::NotTransmitted;
    let mut echo_transmit_timestamp: Instant = Instant::now(); // Need to add assertion that in steady-state this cannot be less than the next cycle time
    loop {
        // Check echo first, then trigger
        let echo = get_echo(&echo_hndl)?;

        // Init condition: Echo LOW, Trig not yet triggered
        if echo == 0 && trig_state == TrigState::NotTriggered {
            set_trig_high(&trig_hndl)?;
            trig_state = TrigState::Triggered;
        }
        
        // Transmitted: Echo HIGH after Trig triggered
        if echo == 1 && trig_state == TrigState::Triggered {
            echo_state = EchoState::Transmitted;
        }

        // Reflected: Echo LOW after Trig triggered
        if echo == 0 && trig_state == TrigState::Triggered {
            echo_state = EchoState::Reflected;
        }

        counter = counter.wrapping_add(1);
        next_time += cycle;
    }


    Ok(())
}

fn set_trig_high(trig_handle: &LineHandle) -> Result<(), Error> {
    trig_handle.set_value(1)?;
    Ok(())
}

fn get_echo(echo_handle: &LineHandle) -> Result<u8, Error> {
    let res= echo_handle.get_value()?;
    Ok(res)
}