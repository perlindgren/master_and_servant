# Master

Server side application examples

---

## Host side

```shell
cargo run --example <app>
```

---

## Examples

In the examples folder you find some host side examples. To cut to the chase, look at the `cmd_crc_cobs_lib` example doing the following.

- A request is first serialized, a crc computed and added to the payload, then sent as a cobs encoded package.

- Buffers are allocated once and re-used.

- The statically computed buffer size guarantees sufficiency.

- Caveat, for the cobs encoding, the output buffer is copied once (memcpy), this could be avoided by returning an iterator at the cost of run-time overhead (and increased complexity). No fix planned.
  
