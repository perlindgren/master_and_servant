//! hello.rs
//!
//! On target `cd servant` run:
//!
//! cargo embed --example uart_cdc_echo --release
//! or
//! cargo embed --example uart_cdc_fast_echo --release
//!
//! On host `cd master` run:
//!
//! cargo run --example hello
//!
//! Prints the echoed data.
//! Assumes that each character sent is echoed.
use master::open;
use std::io::Read;

fn main() {
    let mut port = open().unwrap();
    let data = "hello".as_bytes();
    let mut buf = vec![0u8; data.len()];
    let status = port.write(data);
    println!("Write status: {:?}", status);

    let status = port.read_exact(buf.as_mut_slice());
    println!("Read status: {:?}", status);
    println!("Data received: {:?}", buf);
}
