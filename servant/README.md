# servant - running on the SAM E70 Xplained Ultra

## Examples

- `blinky_rtt`, just to see that flashing and rtt tracing is working. Bridge the jumper closest to the barrel connector.

- `uart_cdc`, simple echo off by one, can be used together with the `hello` example in the `master` crate.

- `uart_cdc_fast`, as above but uses task priorities for better performance.

- `cmd`, showcases ssmarshal based serialization, can be used together with the `cmd` example in the `master` crate.
