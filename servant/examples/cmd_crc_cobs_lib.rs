//! cmd
//!
//! ssmarshal + serde + crc + cobs
//!
//! Run on target: `cd servant`
//! cargo embed --example cmd_crc_cobs_lib --release
//!
//! Run on host: `cd master`
//! cargo run --example cmd_crc_cobs_lib
//!
//! Serial at 9600bps, over programmer or ftdi.
//!     TX PA10
//!     RX PA9
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
    use hal::serial::{uart::*, ExtBpsU32};
    use rtt_target::{rprint, rprintln, rtt_init_print};

    // Application dependencies
    use core::mem::size_of;
    use corncobs::{max_encoded_len, ZERO};
    use master_and_servant::{
        deserialize_crc_cobs, serialize_crc_cobs, EvState, Relay, Request, Response,
    };
    use nb::block;

    const IN_SIZE: usize = max_encoded_len(size_of::<Request>() + size_of::<u32>());
    const OUT_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        tx: Tx<Uart0>,
        rx: Rx<Uart0>,
    }

    #[init()]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let pac = ctx.device;

        pac.WDT.mr.modify(|_r, c| c.wddis().set_bit());
        pac.RSWDT.mr.modify(|_r, c| c.wddis().set_bit());

        rtt_init_print!();
        rprintln!("reset - cmd_crc_cobs_lib");
        for _ in 0..5 {
            for _ in 0..1000_000 {
                cortex_m::asm::nop();
            }
            rprint!(".");
        }
        rprintln!("\ninit done");

        let clocks = Tokens::new((pac.PMC, pac.SUPC, pac.UTMI), &pac.WDT.into());
        // use internal rc oscillator for slow clock
        let slck = clocks.slck.configure_internal();
        // use external xtal as oscillator for main clock
        let mainck = clocks.mainck.configure_external_normal(16.MHz()).unwrap();
        let pck: Pck<Pck4> = clocks.pcks.pck4.configure(&mainck, 3).unwrap();
        let (_hclk, mut mck) = HostClockController::new(clocks.hclk, clocks.mck)
            .configure(
                &mainck,
                &mut Efc::new(pac.EFC, VddioLevel::V3),
                HostClockConfig {
                    pres: HccPrescaler::Div1,
                    div: MckDivider::Div1,
                },
            )
            .unwrap();

        let banka = BankA::new(pac.PIOA, &mut mck, &slck, BankConfiguration::default());

        let tx = banka.pa10.into_peripheral();
        let rx = banka.pa9.into_peripheral();
        let mut uart = Uart::new_uart0(
            pac.UART0,
            (tx, rx),
            UartConfiguration::default(9_600.bps()).mode(ChannelMode::Normal),
            PeripheralClock::Other(&mut mck, &pck),
        )
        .unwrap();

        // Listen to an interrupt event.
        uart.listen_slice(&[Event::RxReady]);

        let (tx, rx) = uart.split();

        (Shared {}, Local { tx, rx }, init::Monotonics())
    }

    #[task(binds=UART0, local = [rx ], priority = 2)]
    fn uart0(ctx: uart0::Context) {
        let uart0::LocalResources { rx } = ctx.local;
        loop {
            match rx.read() {
                Ok(data) => lowprio::spawn(data).unwrap(), // panics if buffer full
                _ => break,
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
            rprintln!("\n-- cobs packet received {:?} --", &in_buf[0..*index]);
            *index = 0;

            match deserialize_crc_cobs::<Request>(in_buf) {
                Ok(cmd) => {
                    rprintln!("cmd {:?}", cmd);
                    let response = match cmd {
                        Request::Set {
                            dev_id,
                            pwm_hi_percentage,
                            relay,
                        } => {
                            rprintln!(
                                "dev_id {}, pwm {}, relay {:?}",
                                dev_id,
                                pwm_hi_percentage,
                                relay,
                            );
                            Response::SetOk // or do we want to return status
                        }
                        Request::Get { dev_id } => {
                            rprintln!("dev_id {}", dev_id);
                            Response::Status {
                                ev_state: EvState::Connected,
                                pwm_hi_val: 3.3,
                                pwm_lo_val: 0.0,
                                pwm_hi_percentage: 50,
                                relay: Relay::A,
                                rcd_value: 1.0,
                                current: 2.0,
                                voltages: 3.0,
                                energy: 4.0,
                                billing_energy: 5.0,
                            }
                        }
                    };
                    rprintln!("response {:?}", response);
                    let to_write = serialize_crc_cobs(&response, out_buf);
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
