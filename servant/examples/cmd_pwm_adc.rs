//! cmd_pwm_adc
//!
//! Run on target: `cd servant`
//! cargo embed --example cmd_pwm_adc--release
//!
//! Run on host: `cd master`
//! cargo run --example cmd_crc_cobs_lib
//!
//! Example for the "home_gnx" board.
//! Demonstrates ssmarshal + serde + crc + cobs.
//!
//! Serial at 9600bps, over programmer or ftdi.
//!     TX PA10
//!     RX PA9
//!
//! Together with PWM toggle and ADC reading
#![no_std]
#![no_main]

use panic_rtt_target as _;

#[rtic::app(device = atsamx7x_hal::pac, peripherals = true, dispatchers = [IXC, I2SC0])]
mod app {
    // Backend dependencies
    use atsamx7x_hal as hal;
    use dwt_systick_monotonic::{DwtSystick, ExtU32};
    use hal::afec::*;
    use hal::clocks::*;
    use hal::efc::*;
    use hal::ehal::adc::OneShot;
    use hal::ehal::digital::v2::ToggleableOutputPin;
    use hal::ehal::serial::{Read, Write};
    use hal::fugit::{Instant, RateExtU32};
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

    #[monotonic(binds = SysTick, default = true)]
    type Mono = DwtSystick<16_000_000>;
    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        afec: Afec<Afec0>,
        adc_pin: Pin<PB3, Input>,
        pwm_pin: Pin<PA0, Output>,
        tx: Tx<Uart0>,
        rx: Rx<Uart0>,
    }

    #[init()]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
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
        let (hclk, mut mck) = HostClockController::new(clocks.hclk, clocks.mck)
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
        let bankb = hal::pio::BankB::new(pac.PIOB, &mut mck, &slck, BankConfiguration::default());

        // serial setup
        let tx = banka.pa10.into_peripheral();
        let rx = banka.pa9.into_peripheral();
        let mut uart = Uart::new_uart0(
            pac.UART0,
            (tx, rx),
            UartConfiguration::default(9_600.bps()).mode(ChannelMode::Normal),
            PeripheralClock::Other(&mut mck, &pck),
        )
        .unwrap();

        // pwm and adc setup
        let afec = Afec::new_afec0(pac.AFEC0, &mut mck).unwrap();
        let adc_pin = bankb.pb3.into_input(PullDir::PullUp);
        let pwm_pin = banka.pa0.into_output(true);

        // monotonic timer
        let mut mono = DwtSystick::new(
            &mut ctx.core.DCB,
            ctx.core.DWT,
            ctx.core.SYST,
            hclk.systick_freq().to_Hz(),
        );
        let now = mono.now();

        // Listen to an interrupt event.
        uart.listen_slice(&[Event::RxReady]);

        let (tx, rx) = uart.split();

        // spawn fist sample directly
        adc_sample::spawn_at(now, now).unwrap();

        (
            Shared {},
            Local {
                afec,
                adc_pin,
                pwm_pin,
                tx,
                rx,
            },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(binds=UART0, local = [rx ], priority = 3)]
    fn uart0(ctx: uart0::Context) {
        rprintln!("- uart 0 -");
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

    // adc_sample task
    #[task(priority = 2, local = [afec, pwm_pin, adc_pin, cnt:u32 = 0])]
    fn adc_sample(ctx: adc_sample::Context, now: Instant<u32, 1, 16_000_000>) {
        let adc_sample::LocalResources {
            afec,
            adc_pin,
            pwm_pin,
            cnt,
        } = ctx.local;

        *cnt = (*cnt + 1) % 999;
        // get sample
        let v: f32 = afec.read(adc_pin).unwrap();

        // toggle
        pwm_pin.toggle().unwrap();
        let one_milli = now + 1.millis();

        adc_sample::spawn_at(one_milli, one_milli).unwrap();

        // log
        if *cnt == 0 {
            rprintln!("PB3 (channel 2) = {:.2}V", v);
        }
    }
}
