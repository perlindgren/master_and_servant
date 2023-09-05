# Master

Server side application examples

---

## Examples

- `hello` This simple application sends "hello" over the serial and reads the echoed data.

  You need to start the `servant` first.

  ```shell
  cargo embed --example uart_cdc_fast --release
  ```

  Once that is up and running you can run `hello`.

  ```shell
  cargo embed --example hello
  ```

- `cmd` This application sends a `Command` request to the target, which processes the command and returns with a `Response`.

  You need to start the `servant` first.

  ```shell
  cargo embed --example cmd --release
  ```

  Once that is up and running you can run `cmd` on the host.

  ```shell
  cargo embed --example cmd
  ```

- `corncobs_test` This example showcase `corncobs` encoding and how a `sshmarshal` serialization can be put into a `cobs` frame.

---

## Notes on `sshmarshal` and `corncobs`

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

The `corncobs` encoder provides `max_encoded_len` as a `const` function. This allows us to statically determine safe space requirements as follows.

```rust
let mut cmd_buf = [0u8; size_of::<Command>()];
let mut out_buf = [0u8; max_encoded_len(size_of::<Command>())];
```

So `cmd_buf` is a safe allocation for serializations of `Command`, and `out_buf` is a safe allocation for cobs encoding of serialized `Command`.

In this case, `cmd_buf` is 20 bytes (including padding), an actual serialization uses only 14 bytes.

`out_buf` is 22 bytes (reserving space for framing OH), while the actual frame is only 16 bytes. That means we only need to send 16 bytes over the serial link to transmit a cobs encoded `Command`. Unless bit errors on the link occurs, send/receive of commands is infallible. This holds similarly for any `Sized` data structure in Rust, under the `enum` restrictions earlier.

While `corncobs` is designed for speed and memory efficiency, validation is not natively supported.
