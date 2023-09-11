use serial2::SerialPort;
use std::io::Result;
use std::time::Duration;

#[cfg(target_os = "linux")]
static COM_PATH: &str = "/dev/ttyACM0";
#[cfg(target_os = "windows")]
static COM_PATH: &str = "COM3";

const TIME_OUT: Duration = Duration::from_millis(1000);

pub fn open() -> Result<SerialPort> {
    let mut port = SerialPort::open(COM_PATH, 9600)?;
    // Needed for windows, but should not hurt on Linux
    port.set_dtr(true)?;
    port.set_rts(true)?;
    port.set_write_timeout(TIME_OUT)?;
    port.set_read_timeout(TIME_OUT)?;

    Ok(port)
}
