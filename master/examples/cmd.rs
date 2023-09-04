use std::io::Read;

use master_and_servant::{Command, Msg};
use serial2::SerialPort;
use std::mem::{size_of, size_of_val};

// On Windows, use something like "COM1".
// For COM ports above COM9, you need to use the win32 device namespace, for example "\\.\COM10" (or "\\\\.\\COM10" with string escaping).
// For more details, see: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file?redirectedfrom=MSDN#win32-device-namespaces

fn main() {
    let mut port = SerialPort::open("/dev/ttyACM0", 9600).unwrap();
    let cmd = Command::Set(0x12, Msg::B(12), 0b001);
    let mut buf = [0u8; size_of::<Command>()];
    let n = ssmarshal::serialize(&mut buf, &cmd).unwrap();

    println!("cdm {:?}, size {}, n {}", cmd, size_of_val(&cmd), n);

    // let b = port.write(&buf[0..n]);
    // println!("{:?}", b);

    // let r = port.read_exact(&mut buf[0..n]);

    // println!("{:?}, {:?}", r, buf);

    let (cmd, _) = ssmarshal::deserialize::<Command>(&buf).unwrap();
    println!("cmd {:?}", cmd);
    loop {}
}
