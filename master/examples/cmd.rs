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
use std::mem::size_of;
use master::open;

type InBuf = [u8; size_of::<Command>()];
type OutBuf = [u8; size_of::<Response>()];

fn main() -> Result<(),  std::io::Error> {
    let mut port = open()?;

    let mut out_buf = [0u8; size_of::<Command>()];
    let mut in_buf = [0u8; size_of::<Response>()];

    let cmd = Command::Set(0x12, Message::B(12), 0b001);
    println!("request {:?}", cmd);
    let response = request(
        &cmd,
        &mut port,
        &mut out_buf,
        &mut in_buf,
    )?;
    println!("response {:?}", response);

    let cmd = Command::Get(0x12, 12, 0b001);
    println!("request {:?}", cmd);
    let response = request(
        &cmd,
        &mut port,
        &mut out_buf,
        &mut in_buf,
    )?;
    println!("response {:?}", response);
    Ok(())
}

fn request(
    cmd: &Command,
    port: &mut SerialPort,
    out_buf: &mut OutBuf,
    in_buf: &mut InBuf,
) -> Result<Response, std::io::Error> {
    let _n = ssmarshal::serialize(out_buf, cmd).unwrap();
    port.write_all(out_buf)?;
    port.read_exact(in_buf)?;
    let (response, _) = ssmarshal::deserialize::<Response>(in_buf).unwrap();
    Ok(response)
}
