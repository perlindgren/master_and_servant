# Master

Server side application

## Hello

This simple application sends "hello" over the serial and reads the echoed data.

You need to start the `servant` first.

```shell
cargo embed --example uart_cdc_fast --release
```

Once that is up and running you can run `hello`.

```shell
cargo embed --example hello
```
