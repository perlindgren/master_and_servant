//! cmd
//!
//! ssmarshal + serde
//! 
//! Run on target: `cd servant`
//! cargo embed --example cmd --release
//! 
//! Run on host: `cd master`
//! cargo run --example cmd
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
    use rtt_target::{rprintln, rtt_init_print};

    // Application dependencies
    use core::mem::size_of;
    use ssmarshal;
    use nb::block;

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
            n: usize = 0, 
            in_buf: [u8; size_of::<Command>()] = [0u8; size_of::<Command>()],
            out_buf: [u8; size_of::<Response>()] = [0u8; size_of::<Response>()]
        ]
    )]
    fn lowprio(ctx: lowprio::Context, data: u8) {
        rprintln!("received : {}", data);
        let lowprio::LocalResources { tx, n, in_buf, out_buf } = ctx.local;
        rprintln!("{} {} {}", data, n, in_buf.len());
        in_buf[*n] = data;
        *n += 1;
        if *n == in_buf.len() {
            rprintln!("command received");
            let (cmd, _) = ssmarshal::deserialize::<Command>(in_buf).unwrap();
            rprintln!("cmd {:?}", cmd);
            *n = 0;

            let response = match cmd {
                Command::Set(_id, _par, _dev) => Response::SetOk,
                Command::Get(id, par, dev) => Response::Data(id, par, 42, dev),
            };

            let _n = ssmarshal::serialize(out_buf, &response).unwrap();
            
            for byte in out_buf {
                block!(tx.write(*byte)).unwrap();
            }
        }
    }
}
