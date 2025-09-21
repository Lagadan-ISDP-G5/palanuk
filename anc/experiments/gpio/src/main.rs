use gpio_cdev::{Chip, LineRequestFlags};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut chip = Chip::new("/dev/gpiochip4")?; // For some reason it's /dev/gpiochip4 on the Pi 5
    let handle = chip
        .get_line(4)? // Pi GPIO4
        .request(LineRequestFlags::INPUT, 0, "read-input")?;

    for _ in 1..4 {
        println!("Value: {:?}", handle.get_value()?);
    }

    Ok(())
}