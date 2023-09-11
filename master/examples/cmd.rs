// cmd.rs
//
// On host `cd master` run:
// cargo run --example cmd
//
// On target `cd servant` run
// cargo embed --example cmd --release 

use master_and_servant::{Command, Message, Response};
use serial2::SerialPort;
use std::io::Read;
use std::mem::{size_of, size_of_val};
use master::open;

type InBuf = [u8; size_of::<Command>()];
type OutBuf = [u8; size_of::<Response>()];

fn main() -> Result<(),  std::io::Error> {
    let mut port = open()?;

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
    Ok(())
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
    println!("{:?}", r);
    let r = port.flush();
    println!("{:?}", r);

    let r = port.read_exact(in_buf);
    println!("{:?}, {:?}", r, in_buf);
    let (response, _) = ssmarshal::deserialize::<Response>(in_buf).unwrap();

    response
}
