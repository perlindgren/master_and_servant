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
//! Together with PWM toggle and ADC reading.
//!
#![no_std]
#![no_main]

use panic_rtt_target as _;

#[rtic::app(device = atsamx7x_hal::pac, peripherals = true, dispatchers = [CCW, CCF, IXC])]
mod app {
    use core::fmt::Write;

    // Backend dependencies
    use atsamx7x_hal as hal;
    use dwt_systick_monotonic::{DwtSystick, ExtU32};
    use hal::afec::*;
    use hal::clocks::*;
    use hal::efc::*;
    use hal::ehal::adc::OneShot;
    use hal::ehal::digital::v2::ToggleableOutputPin;
    use hal::ehal::serial::{self, Read};
    use hal::fugit::{Instant, RateExtU32};
    use hal::generics::events::EventHandler;
    use hal::pio::*;
    use hal::serial::uart::UartConfiguration;
    use hal::serial::{uart::*, ExtBpsU32};
    use rtt_target::{rtt_init, UpChannel};

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
        rtt_0: UpChannel,
        rtt_1: UpChannel,
        rtt_2: UpChannel,

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

        let mut channels = rtt_init!(
            up: {
                0: {
                    size: 128
                    name:"Idle"
                }
                1: {
                    size: 1024
                    // mode: NoBlockTrim
                    name:"Log"
                }
                2: {
                    size: 128
                    name:"Trace"
                }
            }
        );
        let (mut rtt_0, rtt_1, rtt_2) = (channels.up.0, channels.up.1, channels.up.2);

        writeln!(rtt_0, "reset - cmd_crc_cobs_lib").ok();
        for _ in 0..5 {
            for _ in 0..1000_000 {
                cortex_m::asm::nop();
            }
            write!(rtt_0, ".");
        }
        writeln!(rtt_0, "\ninit done");

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
        let bankb = BankB::new(pac.PIOB, &mut mck, &slck, BankConfiguration::default());

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
        // lowprio::spawn(123);

        (
            Shared {},
            Local {
                rtt_0,
                rtt_1,
                rtt_2,

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

    #[task(binds=UART0, local = [rx, rtt_0], priority = 2)]
    fn uart0(ctx: uart0::Context) {
        let uart0::LocalResources { rtt_0, rx } = ctx.local;
        writeln!(rtt_0, "uart_0").ok();
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
            rtt_1,
            tx,
            // locally initialized resources
            index: usize = 0,
            in_buf: [u8; IN_SIZE] = [0u8; IN_SIZE],
            out_buf: [u8; OUT_SIZE] = [0u8; OUT_SIZE]
        ]
    )]
    fn lowprio(ctx: lowprio::Context, data: u8) {
        let lowprio::LocalResources {
            rtt_1,
            tx,
            index,
            in_buf,
            out_buf,
        } = ctx.local;
        writeln!(rtt_1, "in_buf[{}]={}", index, data);
        in_buf[*index] = data;

        // ensure index in range
        if *index < IN_SIZE - 1 {
            *index += 1;
        }

        // end of cobs frame
        if data == ZERO {
            writeln!(
                rtt_1,
                "\n-- cobs packet received {:?} --",
                &in_buf[0..*index]
            );
            *index = 0;

            match deserialize_crc_cobs::<Request>(in_buf) {
                Ok(cmd) => {
                    writeln!(rtt_1, "cmd {:?}", cmd);
                    let response = match cmd {
                        Request::Set {
                            dev_id,
                            pwm_hi_percentage,
                            relay,
                        } => {
                            writeln!(
                                rtt_1,
                                "dev_id {}, pwm {}, relay {:?}",
                                dev_id, pwm_hi_percentage, relay,
                            );
                            Response::SetOk // or do we want to return status
                        }
                        Request::Get { dev_id } => {
                            writeln!(rtt_1, "dev_id {}", dev_id);
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
                    writeln!(rtt_1, "response {:?}", response);
                    let to_write = serialize_crc_cobs(&response, out_buf);
                    use hal::ehal::serial::Write;
                    for byte in to_write {
                        block!(tx.write(*byte)).unwrap();
                    }
                }

                Err(err) => {
                    writeln!(rtt_1, "ssmarshal err {:?}", err);
                }
            }
        }
    }

    // adc_sample task
    #[task(priority = 3, local = [rtt_2, afec, pwm_pin, adc_pin, cnt:u32 = 0])]
    fn adc_sample(ctx: adc_sample::Context, now: Instant<u32, 1, 16_000_000>) {
        let adc_sample::LocalResources {
            rtt_2,
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
            writeln!(rtt_2, "PB3 (channel 2) = {:.2}V", v);
        }
    }
}
