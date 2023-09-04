use master_and_servant::{Command, Message, Response};
use serial2::SerialPort;
use std::io::Read;
use std::mem::{size_of, size_of_val};

type InBuf = [u8; size_of::<Command>()];
type OutBuf = [u8; size_of::<Response>()];

fn main() {
    let mut port = SerialPort::open("/dev/ttyACM0", 9600).unwrap();
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

    // just to prevent shutdown of socket
    loop {}
}

fn request(
    cmd: Command,
    port: &mut SerialPort,
    out_buf: &mut OutBuf,
    in_buf: &mut InBuf,
) -> Response {
    let n = ssmarshal::serialize(out_buf, &cmd).unwrap();
    println!("cdm {:?}, size {}, n {}", cmd, size_of_val(&cmd), n);

    let b = port.write(out_buf);
    println!("{:?}", b);

    let r = port.read_exact(in_buf);
    println!("{:?}, {:?}", r, in_buf);
    let (response, _) = ssmarshal::deserialize::<Response>(in_buf).unwrap();

    response
}
