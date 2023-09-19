# servant - running on the SAM E70 Xplained Ultra

## Requirements

- thumbv7em-none-eabihf

  `rustup target add thumbv7em-none-eabihf`

## Target side

```shell
cargo embed --example <app>
```
  
## Examples

- `blinky`, just to see that flashing and rtt tracing is working.

- `cmd_crc_cobs_lib`, simple echo off by one, can be used together with the `hello` example in the `master` crate.

  On the host side, run some terminal application to send characters.

  Under Linux/Windows, in the `master folder`:

  ```shell
  cargo embed cmd_crc_cobs_lib
  ```

---

## Tooling

- [probe-rs](https://probe.rs/)

  Install `probe-rs` and the `vscode` plugin for debugging.

  ```shell
  cargo embed <app>
  ```

  Or run in `vscode`.

## Keybindings

- `<CTRL-D>` for debug mode: Choose profile in dropdown.
- `<F5>` for starting debugging. (Will compile, flash etc.)
- `<Ctrl-B>` to build/check/clippy.

- `.vscode`
  - `tasks.json`, the build profiles.
  - `launch.json`, the launch profiles and `probe-rs-debug` config, with RTT settings etc.
