# Master and Servant

A POC implementation for communication between a single master and potentially several servants over serial (IRDA).

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

### memory.x and the build.rs

The `memory.x` file specifies the memory layout.

The `build.rs` copies that to the `out` folder for the `servant` crate, and extends the link search to include this `out` folder.

### cargo embed

To run the `blinky_rtt` example from the `servant` folder:

```shell
cargo embed --example blinky_rtt
```
