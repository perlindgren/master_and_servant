// cmd_cobs.rs
//
// On host `cd master` run:
// cargo run --example cmd_cobs
//
// On target `cd servant` run
// cargo embed --example cmd_cobs --release

use corncobs::{decode_in_place, encode_buf, max_encoded_len, ZERO};
use master::open;
use master_and_servant::{Command, Message, Response};
use serial2::SerialPort;
use std::io::Read;
use std::mem::size_of;

const IN_SIZE: usize = max_encoded_len(size_of::<Response>());
const OUT_SIZE: usize = max_encoded_len(size_of::<Command>());

type InBuf = [u8; IN_SIZE];
type OutBuf = [u8; OUT_SIZE];

fn main() -> Result<(), std::io::Error> {
    let mut port = open()?;

    let mut out_buf = [0u8; OUT_SIZE];
    let mut in_buf = [0u8; IN_SIZE];

    let cmd = Command::Set(0x12, Message::B(12), 0b001);
    println!("request {:?}", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf)?;
    println!("response {:?}", response);

    let cmd = Command::Get(0x12, 12, 0b001);
    println!("request {:?}", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf)?;
    println!("response {:?}", response);
    Ok(())
}

fn request(
    cmd: &Command,
    port: &mut SerialPort,
    out_buf: &mut OutBuf,
    in_buf: &mut InBuf,
) -> Result<Response, std::io::Error> {
    let n = ssmarshal::serialize(out_buf, cmd).unwrap();
    println!("ser n {}", n);
    println!("ser {:?}", &out_buf[0..n]);

    let buf_copy = out_buf.clone(); // could we do better?
    let n = encode_buf(&buf_copy[0..n], out_buf);
    println!("cobs n {}", n);
    println!("out_buf {:?}", &out_buf[0..n]);

    port.write_all(&out_buf[0..n])?;

    let mut index: usize = 0;
    loop {
        let slice = &mut in_buf[index..index + 1];
        if index < IN_SIZE {
            index += 1;
        }
        port.read_exact(slice)?;
        if slice[0] == ZERO {
            println!("ZERO");
            break;
        }
    }
    println!("cobs index {}", index);

    let n = decode_in_place(in_buf).unwrap();
    println!("ser # decoded {}", n);

    let (response, n) = ssmarshal::deserialize::<Response>(&in_buf[0..n]).unwrap();
    println!("ser used {}", n);
    Ok(response)
}
