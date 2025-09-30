use gpio_cdev::{Chip, Error, EventRequestFlags, LineHandle, Line, LineRequestFlags, EventType};
use std::{thread::sleep, time::*};

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

pub struct HcSr04 {
    trig: LineHandle,
    echo: Line,
}

/// Doesn't really seem to work
pub fn range_to_timeout(range: DistanceUnit) -> Result<Duration, String> {
    let res = match range {
        DistanceUnit::Cm(val) => (val / 200.0) / SPEED_OF_SOUND.to_val(),
        DistanceUnit::Mm(_) => return Err("range must be in cm".to_string())
    };
    Ok(Duration::from_secs_f64(res))
}

impl HcSr04 {
    pub fn new(trig: u32, echo: u32) -> Result<Self, Error> {
        let mut chip = Chip::new("/dev/gpiochip4")?;

        Ok(Self {
            trig: chip.get_line(trig)?
                    .request(LineRequestFlags::OUTPUT, 0, "hc-sr04-trigger")?,
            echo: chip.get_line(echo)?
        })
    }

    /// Returns distance in cm by default
    fn dist(&mut self, timeout: Option<Duration>) -> Result<f64, Error> {
        self.trig.set_value(0)?;
        std::thread::sleep(Duration::from_micros(2));
        self.trig.set_value(1)?;
        std::thread::sleep(Duration::from_micros(10));
        self.trig.set_value(0)?;

        let mut tx_time = Instant::now();
        let mut dist: DistanceUnit = DistanceUnit::Cm(0.0);
        let mut polling = true;

        while polling {
            match timeout {
                Some(timeout) => sleep(timeout),
                None => {}
            }
    
            let events = self.echo.events(LineRequestFlags::INPUT, EventRequestFlags::BOTH_EDGES, "hc-sr04-echo")?;        

            for event in events {
                match event?.event_type() {
                    EventType::RisingEdge => {
                        tx_time = Instant::now();
                    },
                    EventType::FallingEdge => {
                        let tof: Duration = Instant::now() - tx_time;
                        dist.write_val(50.0*(SPEED_OF_SOUND.to_val() * tof.as_secs_f64()));
                        polling = false;
                        break;
                    }
                }
            }
        }
        Ok(dist.to_val())
    }

    pub fn dist_cm(&mut self, timeout: Option<Duration>) -> Result<DistanceUnit, Error> {
        let res = self.dist(timeout)?;
        Ok(DistanceUnit::Cm(res))
    }

    pub fn dist_mm(&mut self, timeout: Option<Duration>) -> Result<DistanceUnit, Error> {
        let res = self.dist(timeout)?;
        Ok(DistanceUnit::Mm(10.0*res))
    }
}