use corncobs::{encode_buf, max_encoded_len};
use master_and_servant::{Command, Message};
use std::mem::{size_of, size_of_val};

fn main() {
    let in_buf = [0, 0, 0u8];
    let mut out_buf = [0u8; 256];

    let n = encode_buf(&in_buf, &mut out_buf);
    println!("n {}, out_buf {:#x?}", n, &out_buf[0..n]);

    println!("size {}", max_encoded_len(in_buf.len()));

    // test make packet
    let mut cmd_buf = [0u8; size_of::<Command>()];
    let mut out_buf = [0u8; max_encoded_len(size_of::<Command>())];

    let cmd = Command::Set(0x12, Message::B(12), 0b001);
    let n_cmd = ssmarshal::serialize(&mut cmd_buf, &cmd).unwrap();
    println!("n_cmd {}, size cmd_buf {}", n_cmd, size_of_val(&cmd_buf));

    let n_out = encode_buf(&cmd_buf[0..n_cmd], &mut out_buf);
    println!("n_out {}, size out_buf {}", n_out, size_of_val(&out_buf));

    println!(
        "cmd_buf {:?}\nout_buf {:?}",
        &cmd_buf[0..n_cmd],
        &out_buf[0..n_out]
    )
}
