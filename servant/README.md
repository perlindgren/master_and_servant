# servant - running on the SAM E70 Xplained Ultra

## Requirements

- thumbv7em-none-eabihf

  `rustup target add thumbv7em-none-eabihf`
  
## Examples

- `blinky_rtt`, just to see that flashing and rtt tracing is working. Bridge the jumper closest to the barrel connector.

- `uart_cdc_echo`, simple echo off by one, can be used together with the `hello` example in the `master` crate.

  On the host side, run some terminal application to send characters.

  Under linux: e.g.,
  `minicom -b 9600 -D /dev/ttyACM0`

  Under Windows: e.g.,
  `CoolTerm` connected to COM3 at 9600 8N1.

  Actual `tty`/`COM` port might vary.

- `uart_cdc_fast_echo`, as above but uses task priorities for better performance, delegating tracing to a low priority task.

- `cmd`, showcases ssmarshal based serialization, can be used together with the `cmd` example in the `master` crate.
