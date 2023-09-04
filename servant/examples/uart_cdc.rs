//! USART CDC
//!
//! Ports used
//! PA21 CDC_USART_RX <-> EDBG_UART_TXD
//! PB04 CDC_USART_TX <-> EDBG_UART_RXD
//!
//! On the host side run some terminal e.g.,
//! minicom -b 9600 -D /dev/ttyACM0
//!
//! The application will echo back the character +1 (a->b, etc.).
//!
//! Due to unknown reasons, only 9600 bps seems to work.
#![no_std]
#![no_main]

use panic_rtt_target as _;

#[rtic::app(device = atsamx7x_hal::pac, peripherals = true)]
mod app {
    use atsamx7x_hal as hal;
    use hal::clocks::*;
    use hal::efc::*;
    use hal::ehal::prelude::*;
    use hal::ehal::serial::{Read, Write};
    use hal::fugit::RateExtU32;
    use hal::generics::events::EventHandler;
    use hal::pio::*;
    use hal::serial::uart::UartConfiguration;
    use hal::serial::usart::Event;
    use hal::serial::{usart::*, ExtBpsU32};
    use rtt_target::{rprintln, rtt_init_print};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        uart: Uart<Usart1>,
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
        let pck: Pck<Pck4> = clocks.pcks.pck4.configure(&mainck, 1).unwrap();

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
        let mut uart = handles
            .uart
            .configure(&usart, &mck, UartConfiguration::default(9600.bps()))
            .unwrap();

        // Listen to an interrupt event.
        // usart.listen_slice(&[Event::RxReady, Event::TxReady]); to listen also for TxReady
        usart.listen_slice(&[Event::RxReady]);

        usart.enter_mode(&uart);
        uart.write(b's'); // .unwrap();

        (Shared {}, Local { uart, usart }, init::Monotonics())
    }

    #[task(binds=USART1, local = [uart, usart], priority = 2)]
    fn usart(ctx: usart::Context) {
        rprintln!("interrupt");
        use hal::serial::usart::Event::*;

        let usart::LocalResources { uart, usart } = ctx.local;
        for event in usart.events() {
            match event {
                RxReady => {
                    let data = uart.read().unwrap();
                    rprintln!("read {:?}", data);
                    uart.write(data + 1);
                }
                TxReady => {
                    // uart.write(b'r');
                }
                TxEmpty => {
                    // uart.write(b'e');
                }
                _ => {
                    rprintln!("event {:?}", event);
                    uart.clear_errors();
                }
            }
        }
    }
}
