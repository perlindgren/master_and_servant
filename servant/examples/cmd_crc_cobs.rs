//! cmd
//!
//! ssmarshal + serde + crc + cobs
//!
//! Run on target: `cd servant`
//! cargo embed --example cmd_crc_cobs --release
//!
//! Run on host: `cd master`
//! cargo run --example cmd_crc_cobs
//!
#![no_std]
#![no_main]

use panic_rtt_target as _;

#[rtic::app(device = atsamx7x_hal::pac, peripherals = true, dispatchers = [IXC])]
mod app {
    // Backend dependencies
    use atsamx7x_hal as hal;
    use hal::clocks::*;
    use hal::efc::*;
    use hal::ehal::serial::{Read, Write};
    use hal::fugit::RateExtU32;
    use hal::generics::events::EventHandler;
    use hal::pio::*;
    use hal::serial::uart::UartConfiguration;
    use hal::serial::usart::Event;
    use hal::serial::{usart::*, ExtBpsU32};
    use master_and_servant::{Command, Response};
    use rtt_target::{rprint, rprintln, rtt_init_print};

    // Application dependencies
    use core::mem::size_of;
    use corncobs::{decode_in_place, encode_buf, max_encoded_len, ZERO};
    use crc::{Crc, CRC_32_CKSUM};
    use nb::block;
    use ssmarshal;

    pub const CKSUM: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

    const IN_SIZE: usize = max_encoded_len(size_of::<Command>() + size_of::<u32>());
    const OUT_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        tx: Tx<Usart1>,
        rx: Rx<Usart1>,
        usart: Usart<Usart1>,
    }

    #[init()]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("init");
        let pac = ctx.device;

        let clocks = Tokens::new((pac.PMC, pac.SUPC, pac.UTMI), &pac.WDT.into());
        let slck = clocks.slck.configure_external_normal();
        let mainck = clocks.mainck.configure_external_normal(12.MHz()).unwrap();
        let mut efc = Efc::new(pac.EFC, VddioLevel::V3);
        let (_hclk, mut mck) = HostClockController::new(clocks.hclk, clocks.mck)
            .configure(
                &mainck,
                &mut efc,
                HostClockConfig {
                    pres: HccPrescaler::Div1,
                    div: MckDivider::Div1,
                },
            )
            .unwrap();
        let _pck: Pck<Pck4> = clocks.pcks.pck4.configure(&mainck, 1).unwrap();

        let banka = BankA::new(pac.PIOA, &mut mck, &slck, BankConfiguration::default());
        let bankb = BankB::new(pac.PIOB, &mut mck, &slck, BankConfiguration::default());

        // usart1
        let miso = banka.pa21.into_peripheral(); // RXD1
        let mosi = bankb.pb4.into_peripheral(); // TXD1
        let clk = banka.pa23.into_peripheral(); // CKL1
        let nss = banka.pa24.into_peripheral(); // RTS1?

        // Create the top-level USART abstraction
        let (handles, mut usart) = Usart::new_usart1(pac.USART1, (mosi, miso, clk, nss), &mut mck);

        // consume the usart token and turn it into a uart
        let uart = handles
            .uart
            .configure(&usart, &mck, UartConfiguration::default(9600.bps()))
            .unwrap();

        // Listen to an interrupt event.
        // usart.listen_slice(&[Event::RxReady, Event::TxReady]); to listen also for TxReady
        usart.listen_slice(&[Event::RxReady]);

        usart.enter_mode(&uart);
        let (tx, rx) = uart.split();

        (Shared {}, Local { tx, rx, usart }, init::Monotonics())
    }

    #[task(binds=USART1, local = [rx, usart], priority = 2)]
    fn usart(ctx: usart::Context) {
        use hal::serial::usart::Event::*;

        let usart::LocalResources { rx, usart } = ctx.local;
        for event in usart.events() {
            match event {
                RxReady => {
                    let data = rx.read().unwrap();
                    let _ = lowprio::spawn(data);
                }
                TxReady => {
                    // uart.write(b'r');
                }
                TxEmpty => {
                    // uart.write(b'e');
                }
                _ => {
                    rprintln!("event {:?}", event);
                    rx.clear_errors();
                }
            }
        }
    }

    #[task(
        priority = 1,
        capacity = 100,
        local = [
            tx,
            // locally initialized resources
            index: usize = 0,
            in_buf: [u8; IN_SIZE] = [0u8; IN_SIZE],
            out_buf: [u8; OUT_SIZE] = [0u8; OUT_SIZE]
        ]
    )]
    fn lowprio(ctx: lowprio::Context, data: u8) {
        let lowprio::LocalResources {
            tx,
            index,
            in_buf,
            out_buf,
        } = ctx.local;
        rprint!("r{} {}", data, index);
        in_buf[*index] = data;

        // ensure index in range
        if *index < IN_SIZE - 1 {
            *index += 1;
        }

        // end of cobs frame
        if data == ZERO {
            rprintln!("cobs frame received index {}", index);
            rprintln!("in_buf {:?}", &in_buf[0..*index]);

            *index = 0; // reset index
            let n = decode_in_place(in_buf).unwrap();
            rprintln!("decoded n {}, {:?}", n, &in_buf[0..n]);

            match ssmarshal::deserialize::<Command>(in_buf) {
                Ok((cmd, cmd_used)) => {
                    rprintln!("cmd {:?} cmd_used {}", cmd, cmd_used);
                    let crc_buf = &in_buf[cmd_used..];
                    rprintln!("crc_buf {:?}", crc_buf);
                    let (crc, crc_used) = ssmarshal::deserialize::<u32>(crc_buf).unwrap();
                    rprintln!("crc {}, crc_used {}", crc, crc_used);

                    let cmd_crc = CKSUM.checksum(&in_buf[0..cmd_used]);
                    rprintln!("cmd_crc {}, valid {}", cmd_crc, cmd_crc == crc);

                    let response = match cmd {
                        Command::Set(_id, _par, _dev) => Response::SetOk,
                        Command::Get(id, par, dev) => Response::Data(id, par, 42, dev),
                    };

                    rprintln!("response {:?}", response);
                    let resp_used = ssmarshal::serialize(out_buf, &response).unwrap();
                    rprintln!("resp_used {}", resp_used);

                    let resp_crc = CKSUM.checksum(&out_buf[0..resp_used]);
                    rprintln!("resp_crc {}", resp_crc);

                    let crc_used =
                        ssmarshal::serialize(&mut out_buf[resp_used..], &resp_crc).unwrap();
                    rprintln!("crc_used {}", crc_used);

                    let buf_clone = out_buf.clone();
                    rprintln!(
                        "n {}, out_buf {:?}, out_buf_clone {:?}",
                        n,
                        &out_buf[0..resp_used + crc_used],
                        &buf_clone[0..n]
                    );

                    let n = encode_buf(&buf_clone[0..resp_used + crc_used], out_buf);
                    rprintln!("cobs n {}", n);
                    let to_write = &out_buf[0..n];
                    rprintln!("to_write {:?}", to_write);

                    for byte in to_write {
                        block!(tx.write(*byte)).unwrap();
                    }
                }

                Err(err) => {
                    rprintln!("ssmarshal err {:?}", err);
                }
            }
        }
    }
}
