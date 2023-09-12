# Master and Servant

A POC implementation for communication between a single master and potentially several servants over serial (IRDA).

## Resources

- [SAM E70 Xplained Ultra User's Guide](https://ww1.microchip.com/downloads/en/DeviceDoc/SAME70_Xplained_Ultra_Evaluation_User%27s%20Guide_DS70005389B.pdf)

## Master

See repo for additional info.

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

---

### memory.x and the build.rs

The `memory.x` file specifies the memory layout.

The `build.rs` copies that to the `out` folder for the `servant` crate, and extends the link search to include this `out` folder.

---

### cargo embed

To run the `blinky_rtt` example from the `servant` folder:

```shell
cargo embed --example blinky_rtt
```

---

## EDBG

Pin mapping for the `e70q21b` on the Xplained Ultra:

- `PA21 CDC_USART_RX` <- `EDBG_UART_TXD`
- `PB04 CDC_USART_TX` -> `EDBG_UART_RXD`

This is supported by the USART1 peripheral.

---

## `ssmarshal`, `serde`, `serde-derive`, `crc`, `corncobs` 

Data types in Rust are typically `Sized`, in fact a variable in Rust always has a known size, whereas dynamic sized data are represented through references (e.g, `Box`).

For our purposes here, sending/receiving data, `Commands`/`Requests` do not carry references, and thus `Sized`.

`ssmarshal` provides lightweight serialization as follows:

The format is not incredibly compact, but doesn't add extra fluff, and is
quick to en/decode.

- bool is serialized as a byte, 1 if true, 0 is false.
- the integer types are encoded in their little-endian form.
- f32 is bitcast to a u32 then encoded as a u32. likewise, f64 and u64.
- inhabited enums are serialized as 1 byte for the discriminant, and then the fields.
- structs are serialized as just their fields.
- the unit type and uninhabited enums are not serialized at all.
- tuples are serialized as the fields, in order.

Rust data types are defined with padding, adding additional space to the layout, `ssmarshal` is non-padded, and each data type is encoded 1-1 to the Rust representation, or cheaper (due to the lack of padding).

Notice that `enums` must use the `#[repr(C)]` (and the number of variants is limited to 256). These invariants are up to you to ensure!

The `corncobs` encoder provides `max_encoded_len` as a `const` function. This allows us to statically determine safe space requirements, e.g, as follows.

```rust
const IN_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());
const OUT_SIZE: usize = max_encoded_len(size_of::<Command>() + size_of::<u32>());

type InBuf = [u8; IN_SIZE];
type OutBuf = [u8; OUT_SIZE];
```

So `InBuf` is a safe allocation for serializations of `Response`, and `OutBuf` is a safe allocation for `Command`, in both cases includes padding, checksum `<u32>`, and cobs encoding.

Unless bit errors on the link occurs, send/receive of commands are infallible. This holds similarly for any `Sized` data structure in Rust, under the `enum` restrictions discussed earlier.

While `corncobs` is designed for speed and memory efficiency, validation is not natively supported. To this end we adopt the `crc` crate. The `crc` is computed on serialization and part of the cobs encoding package. 

The approach is similar to `postcard`, but a bit more bare bones (without `varint` dependency). For buffer types created as shown above, allocations are guaranteed. 

---

## Future work

Current stable Rust does not allow buffer types to be computed at compile time, it may be possible by nightly features, but out of scope for this work.

A possible workaround is to implement a custom derive, providing a buffer type and buffer type constructor.


