use gpio_cdev::{Chip, Error, EventRequestFlags, LineHandle, LineRequestFlags};
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

    let mut chip = Chip::new("/dev/gpiochip4")?; // For some reason it's /dev/gpiochip4 on the Pi 5
    let trig_hndl = chip
        .get_line(HC_SR04_TRIG)?
        .request(LineRequestFlags::OUTPUT, 0, "hc-sr04-trigger")?;

    let echo_hndl = chip.get_line(HC_SR04_ECHO)?;

    for event in echo_hndl.events(
        LineRequestFlags::INPUT,
        EventRequestFlags::BOTH_EDGES,
        "hc-sr04-echo")? {
        let evt = event?;
        match evt.event_type() {
            EventType::RisingEdge => {
                output
            }
        }
    };

    // All of this data should really be stored in a struct, exposed by a library
    let mut trig_state: TrigState = TrigState::NotTriggered;
    let mut echo_state: EchoState = EchoState::NotTransmitted;
    let mut echo_transmit_timestamp: Instant = Instant::now(); // Need to add assertion that in steady-state this cannot be less than the next cycle time
    let mut echo_reflect_timestamp: Instant = Instant::now();
    let mut time_of_flight: Duration = Duration::from_millis(0);
    let mut distance: DistanceUnit = DistanceUnit::Cm(0.0);
    let mut counter: u16 = 0;

    let mut moving_average: Moving<f64> = Moving::new();
    loop {
        // Check echo first, then trigger
        let mut echo = get_echo(&echo_hndl)?;

        // Init condition: Echo LOW, Trig not yet triggered
        if echo == 0 && trig_state == TrigState::NotTriggered && (echo_state == EchoState::Reflected || echo_state == EchoState::NotTransmitted) {
            set_trig_high(&trig_hndl, Duration::from_micros(10))?;
            trig_state = TrigState::Triggered;
            // log::info!("Triggering...");
        }

        echo = get_echo(&echo_hndl)?;
        // Transmitted: Echo HIGH after Trig triggered
        if echo == 1 && trig_state == TrigState::Triggered {
            echo_transmit_timestamp = Instant::now();
            echo_state = EchoState::Transmitted;
            // log::info!("Transmitted")
        }

        echo = get_echo(&echo_hndl)?;
        // Reflected: Echo LOW after Trig triggered
        if echo == 0 && trig_state == TrigState::Triggered {
            echo_reflect_timestamp = Instant::now();
            time_of_flight = echo_reflect_timestamp - echo_transmit_timestamp;
            let dist = 100.0 * (((time_of_flight.as_micros() as f64 / 1_000_000.0) * SPEED_OF_SOUND.to_val()) / 2.0);
            distance.write_val(dist);
            moving_average.add(dist);

            echo_state = EchoState::Reflected;
            trig_state = TrigState::NotTriggered;
            if counter % 4096 == 0 {
                // log::info!("ToF (us): {:05.2}, Distance (cm): {:05.2}", time_of_flight.as_micros(), distance.to_val());
                log::info!("Filtered distance: {:05.2}cm", moving_average)
            }
        }

        counter = counter.wrapping_add(1);
    }
    Ok(())
}

/// Blocks for duration specified by `delay`. `delay` must be at least 10us.
fn set_trig_high(trig_handle: &LineHandle, delay: Duration) -> Result<(), Error> {
    assert!(delay >= Duration::from_micros(10));
    trig_handle.set_value(0)?;
    std::thread::sleep(Duration::from_micros(2));
    trig_handle.set_value(1)?;
    std::thread::sleep(delay);
    trig_handle.set_value(0)?;
    Ok(())
}

fn get_echo(echo_handle: &LineHandle) -> Result<u8, Error> {
    let res= echo_handle.get_value()?;
    Ok(res)
}