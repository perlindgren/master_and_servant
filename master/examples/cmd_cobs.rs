use core::time::Duration;
use master_and_servant::{Command, Message, Response};
use serial2::SerialPort;
use std::io::Read;
use std::mem::{size_of, size_of_val};

type InBuf = [u8; size_of::<Command>()];
type OutBuf = [u8; size_of::<Response>()];

#[cfg(target_os = "linux")]
static COM_PATH: &str = "/dev/ttyACM0";
#[cfg(target_os = "windows")]
static COM_PATH: &str = "COM3";

fn main() {
    let mut port = SerialPort::open(COM_PATH, 9600).unwrap();
    port.set_dtr(true).unwrap();
    port.set_rts(true).unwrap();
    let t = port.get_write_timeout();
    println!("get write timeout t {:?}", t);
    let t = port.set_write_timeout(Duration::from_millis(1000));
    println!("set timeout t {:?}", t);
    let t = port.get_write_timeout();
    println!("get write timeout t {:?}", t);

    let t = port.get_read_timeout();
    println!("get read timeout t {:?}", t);
    let t = port.set_read_timeout(Duration::from_millis(1000));
    println!("get read timeout t {:?}", t);
    let t = port.get_read_timeout();
    println!("get read timeout t {:?}", t);

    let mut out_buf = [0u8; size_of::<Command>()];
    let mut in_buf = [0u8; size_of::<Response>()];

    let response = request(
        Command::Set(0x12, Message::B(12), 0b001),
        &mut port,
        &mut out_buf,
        &mut in_buf,
    );
    println!("response {:?}", response);
    let response = request(
        Command::Get(0x12, 12, 0b001),
        &mut port,
        &mut out_buf,
        &mut in_buf,
    );
    println!("response {:?}", response);
}

fn request(
    cmd: Command,
    port: &mut SerialPort,
    out_buf: &mut OutBuf,
    in_buf: &mut InBuf,
) -> Response {
    let n = ssmarshal::serialize(out_buf, &cmd).unwrap();
    println!("cdm {:?}, size {}, n {}", cmd, size_of_val(&cmd), n);

    let r = port.write_all(out_buf);
    // println!("{:?}", r);
    // let r = port.flush();
    // println!("{:?}", r);

    let r = port.read_exact(in_buf);
    println!("{:?}, {:?}", r, in_buf);
    let (response, _) = ssmarshal::deserialize::<Response>(in_buf).unwrap();

    response
}
