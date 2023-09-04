# Master and Servant

A POC implementation for communication between a single master and potentially several servants over serial (IRDA).

## Resources

- [SAM E70 Xplained Ultra User's Guide](https://ww1.microchip.com/downloads/en/DeviceDoc/SAME70_Xplained_Ultra_Evaluation_User%27s%20Guide_DS70005389B.pdf)

## Master

## Servant

To build examples from the top level workspace.

```shell
cargo build -p servant --example blinky_rtt --target thumbv7em-none-eabihf
```

To build examples from within the `servant` folder.

```shell
cargo build --example blinky_rtt
```

The `-p` (package) and `--target` is not needed here as we are in the member folder, and a local `.cargo/config.toml` sets the default target.

To run the CDC example:

```shell
cargo embed --example uart_cdc --release
```

On the host connect to /dev/ttyACM0 (or similar) with 9600 8N1

```shell
 minicom -b 9600 -D /dev/ttyACM0
```

The application will echo back the character +1 (a -> b, etc.)

### memory.x and the build.rs

The `memory.x` file specifies the memory layout.

The `build.rs` copies that to the `out` folder for the `servant` crate, and extends the link search to include this `out` folder.

### cargo embed

To run the `blinky_rtt` example from the `servant` folder:

```shell
cargo embed --example blinky_rtt
```
