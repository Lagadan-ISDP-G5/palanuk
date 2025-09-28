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

pub enum DistanceUnit {
    Mm(f64),
    Cm(f64),
}

impl DistanceUnit {
    pub fn write_val(&mut self, new_val: f64) {
        match self {
            DistanceUnit::Mm(val) => *val = new_val,
            DistanceUnit::Cm(val) => *val = new_val,
        }
    }

    pub fn to_val(&self) -> f64 {
        match self {
            DistanceUnit::Mm(val) => *val,
            DistanceUnit::Cm(val) => *val,
        }
    }
}

pub enum VelocityUnit {
    MetersPerSecs(f64),
    CentimeterPerSecs(f64),
}

impl VelocityUnit {
    pub fn to_val(&self) -> f64 {
        match self {
            VelocityUnit::MetersPerSecs(val) => *val,
            VelocityUnit::CentimeterPerSecs(val) => *val,
        }
    }
}

const SPEED_OF_SOUND: VelocityUnit = VelocityUnit::MetersPerSecs(343.0);

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

    // All of this data should really be stored in a struct, exposed by a library
    let mut trig_state: TrigState = TrigState::NotTriggered;
    let mut echo_state: EchoState = EchoState::NotTransmitted;
    let mut echo_transmit_timestamp: Instant = Instant::now(); // Need to add assertion that in steady-state this cannot be less than the next cycle time
    let mut echo_reflect_timestamp: Instant = Instant::now();
    let mut time_of_flight: Duration = Duration::from_millis(0);
    let mut distance: DistanceUnit = DistanceUnit::Cm(0.0);
    loop {
        // Check echo first, then trigger
        let mut echo = get_echo(&echo_hndl)?;

        // Init condition: Echo LOW, Trig not yet triggered
        if echo == 0 && trig_state == TrigState::NotTriggered {
            set_trig_high(&trig_hndl, Duration::from_micros(10))?;
            trig_state = TrigState::Triggered;
        }

        echo = get_echo(&echo_hndl)?;

        // Transmitted: Echo HIGH after Trig triggered
        if echo == 1 && trig_state == TrigState::Triggered {
            echo_transmit_timestamp = Instant::now();
            echo_state = EchoState::Transmitted;
            log::info!("echo is 1")
        }

        std::thread::sleep(Duration::from_micros(300)); // Wait for reflection
        echo = get_echo(&echo_hndl)?;

        // Reflected: Echo LOW after Trig triggered
        if echo == 0 && trig_state == TrigState::Triggered {
            echo_reflect_timestamp = Instant::now();
            time_of_flight = echo_reflect_timestamp - echo_transmit_timestamp;
            let dist = 100.0 * ((time_of_flight.as_secs() as f64 * SPEED_OF_SOUND.to_val()) / 2.0);
            distance.write_val(dist);

            echo_state = EchoState::Reflected;
            trig_state = TrigState::NotTriggered;
        }

        log::info!("echo: {}, ToF (s): {}, Distance (cm): {}", echo, time_of_flight.as_secs(), distance.to_val());
        // std::thread::sleep(Duration::from_millis(10));
    }


    Ok(())
}

/// Blocks for duration specified by `delay`. `delay` must be at least 10us.
fn set_trig_high(trig_handle: &LineHandle, delay: Duration) -> Result<(), Error> {
    assert!(delay >= Duration::from_micros(10));
    trig_handle.set_value(1)?;
    std::thread::sleep(delay);
    Ok(())
}

fn get_echo(echo_handle: &LineHandle) -> Result<u8, Error> {
    let res= echo_handle.get_value()?;
    Ok(res)
}