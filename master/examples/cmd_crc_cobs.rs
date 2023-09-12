// cmd_cobs.rs
//
// On host `cd master` run:
// cargo run --example cmd_crc_cobs
//
// On target `cd servant` run
// cargo embed --example cmd_crc_cobs --release

use corncobs::{decode_in_place, encode_buf, max_encoded_len, ZERO};
use crc::{Crc, CRC_32_CKSUM};
use master::open;
use master_and_servant::{Command, Message, Response};
use serial2::SerialPort;
use std::io::Read;
use std::mem::size_of;

const IN_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());
const OUT_SIZE: usize = max_encoded_len(size_of::<Command>() + size_of::<u32>());
pub const CKSUM: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

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

    // let cmd = Command::Get(0x12, 12, 0b001);
    // println!("request {:?}", cmd);
    // let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf)?;
    // println!("response {:?}", response);
    Ok(())
}

fn request(
    cmd: &Command,
    port: &mut SerialPort,
    out_buf: &mut OutBuf,
    in_buf: &mut InBuf,
) -> Result<Response, std::io::Error> {
    println!("out_buf {}", out_buf.len());
    let n_cmd = ssmarshal::serialize(out_buf, cmd).unwrap();
    println!("ser n {}", n_cmd);
    println!("ser {:?}", &out_buf[0..n_cmd]);

    let crc = CKSUM.checksum(&out_buf[0..n_cmd]);
    println!("crc {}", crc);

    let n_crc = ssmarshal::serialize(&mut out_buf[n_cmd..], &crc).unwrap();
    println!("n_crc {}", n_crc);

    let buf_copy = out_buf.clone(); // could we do better?
    let n = encode_buf(&buf_copy[0..n_cmd + n_crc], out_buf);
    println!("cobs n {}", n);
    let to_write = &out_buf[0..n];
    println!("to_write {:?}", to_write);

    port.write_all(to_write)?;

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

    let (response, resp_used) = ssmarshal::deserialize::<Response>(&in_buf[0..n]).unwrap();
    println!("reps used {}", resp_used);

    let crc_buf = &in_buf[resp_used..];
    println!("crc_buf {:?}", crc_buf);
    let (crc, crc_used) = ssmarshal::deserialize::<u32>(crc_buf).unwrap();
    println!("crc {}, crc_used {}", crc, crc_used);

    let resp_crc = CKSUM.checksum(&in_buf[0..resp_used]);
    println!("cmd_crc {}, valid {}", resp_crc, resp_crc == crc);

    Ok(response)
}
