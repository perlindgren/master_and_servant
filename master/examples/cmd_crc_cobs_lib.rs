//! cmd_cobs.rs
//!
//! On target `cd servant` run:
//!
//! cargo embed --example cmd_crc_cobs_lib --release
//!
//! On host `cd master` run:
//! cargo run --example cmd_crc_cobs_lib
//!
use corncobs::{max_encoded_len, ZERO};
use master::open;
use master_and_servant::{deserialize_crc_cobs, serialize_crc_cobs, Relay, Request, Response};
use serial2::SerialPort;
use std::io::Read;
use std::mem::size_of;

const IN_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());
const OUT_SIZE: usize = max_encoded_len(size_of::<Request>() + size_of::<u32>());

type InBuf = [u8; IN_SIZE];
type OutBuf = [u8; OUT_SIZE];

fn main() -> Result<(), std::io::Error> {
    let mut port = open()?;

    let mut out_buf = [0u8; OUT_SIZE];
    let mut in_buf = [0u8; IN_SIZE];

    let cmd = Request::Set {
        dev_id: 0,
        pwm_hi_percentage: 75,
        relay: Relay::B,
    };
    println!("request {:?}", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf);
    println!("response {:?}", response);

    let cmd = Request::Get { dev_id: 3 };
    println!("request {:?}", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf);
    println!("response {:?}", response);
    Ok(())
}

fn request(
    cmd: &Request,
    port: &mut SerialPort,
    out_buf: &mut OutBuf,
    in_buf: &mut InBuf,
) -> Result<Response, std::io::Error> {
    println!("out_buf {}", out_buf.len());
    let to_write = serialize_crc_cobs(cmd, out_buf);
    port.write_all(to_write)?;

    let mut index: usize = 0;
    loop {
        let slice = &mut in_buf[index..index + 1];
        if index < IN_SIZE {
            index += 1;
        }
        port.read_exact(slice)?;
        if slice[0] == ZERO {
            println!("-- cobs package received --");
            break;
        }
    }
    println!("cobs index {}", index);
    Ok(deserialize_crc_cobs(in_buf).unwrap())
}
