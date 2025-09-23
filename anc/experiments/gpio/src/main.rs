use gpio_cdev::{Chip, Error, LineHandle, LineRequestFlags};
use libc::*;
use env_logger::Env;
use core_affinity::*;
use std::{ops::Mul, time::*};

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
    mm(f64),
    cm(f64),
}

impl DistanceUnit {
    pub fn write_val(&mut self, new_val: f64) {
        match self {
            DistanceUnit::mm(val) => *val = new_val,
            DistanceUnit::cm(val) => *val = new_val,
        }
    }

    pub fn to_val(&self) -> f64 {
        match self {
            DistanceUnit::mm(val) => *val,
            DistanceUnit::cm(val) => *val,
        }
    }
}

pub enum VelocityUnit {
    m_per_s(f64),
    cm_per_ms(f64),
}

impl VelocityUnit {
    pub fn to_val(&self) -> f64 {
        match self {
            VelocityUnit::m_per_s(val) => *val,
            VelocityUnit::cm_per_ms(val) => *val,
        }
    }
}

const SPEED_OF_SOUND: VelocityUnit = VelocityUnit::m_per_s(343.0);

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
    let mut echo_reflect_timestamp: Instant = Instant::now();
    let mut time_of_flight: Duration = Duration::from_millis(0);
    let mut distance: DistanceUnit = DistanceUnit::cm(0.0);
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
            echo_transmit_timestamp = Instant::now();
            echo_state = EchoState::Transmitted;
        }

        // Reflected: Echo LOW after Trig triggered
        if echo == 0 && trig_state == TrigState::Triggered {
            echo_reflect_timestamp = Instant::now();
            time_of_flight = echo_reflect_timestamp - echo_transmit_timestamp;
            echo_state = EchoState::Reflected;
        }

        let dist = (time_of_flight.as_secs() as f64 * SPEED_OF_SOUND.to_val()) / 2.0 ;
        distance.write_val(dist);

        log::info!("ToF: {}", time_of_flight.as_millis());
        log::info!("Distance: {}", distance.to_val());

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