use std::io::Read;

use serial2::SerialPort;

// On Windows, use something like "COM1".
// For COM ports above COM9, you need to use the win32 device namespace, for example "\\.\COM10" (or "\\\\.\\COM10" with string escaping).
// For more details, see: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file?redirectedfrom=MSDN#win32-device-namespaces

fn main() {
    let mut port = SerialPort::open("/dev/ttyACM0", 9600).unwrap();
    let hello = "hello".as_bytes();
    let mut buf = vec![0u8; hello.len()];
    let b = port.write(hello);
    println!("{:?}", b);

    let r = port.read_exact(buf.as_mut_slice());

    println!("{:?}, {:?}", r, buf);
    loop {}
}
