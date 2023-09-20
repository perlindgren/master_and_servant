//! blinky
//!
//! Run on target:
//!
//! cargo embed --example blinky
//!
//! Example for the "home_gnx" board.
//! Periodically blinks the board's LED (PA22) at ~1Hz.
//! Log messages are sent using `rtt`.
//! Uses the RTT peripheral for monotonic timer.
//!
#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(device = hal::pac, peripherals = true, dispatchers = [IXC])]
mod app {
    use atsamx7x_hal as hal;
    use hal::clocks::*;
    use hal::efc::*;
    use hal::ehal::digital::v2::ToggleableOutputPin;
    use hal::pio::*;
    use hal::rtt::*;
    use rtt_target::{rprint, rprintln, rtt_init_print};

    #[monotonic(binds = RTT, default = true)]
    type MyMon = Mono<8192>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Pin<PA23, Output>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        ctx.device.WDT.mr.modify(|_r, c| c.wddis().set_bit());
        ctx.device.RSWDT.mr.modify(|_r, c| c.wddis().set_bit());

        rtt_init_print!();
        rprintln!("blinky");
        for _ in 0..10 {
            for _ in 0..4000_000 {
                cortex_m::asm::nop();
            }
            rprint!(".");
        }
        rprintln!("init done");

        let clocks = Tokens::new(
            (ctx.device.PMC, ctx.device.SUPC, ctx.device.UTMI),
            &ctx.device.WDT.into(),
        );

        // use internal rc oscillator for slow clock
        let slck = clocks.slck.configure_internal();
        // use external xtal as oscillator for main clock
        let mainck = clocks.mainck.configure_external_normal(16.MHz()).unwrap();
        let (_hclk, mut mck) = HostClockController::new(clocks.hclk, clocks.mck)
            .configure(
                &mainck,
                &mut Efc::new(ctx.device.EFC, VddioLevel::V3),
                HostClockConfig {
                    pres: HccPrescaler::Div1,
                    div: MckDivider::Div1,
                },
            )
            .unwrap();

        let banka = hal::pio::BankA::new(
            ctx.device.PIOA,
            &mut mck,
            &slck,
            BankConfiguration::default(),
        );
        let led = banka.pa23.into_output(true);

        let mono = Rtt::new_8192Hz(ctx.device.RTT, &slck).into_monotonic();

        toggle_led::spawn().unwrap();

        (Shared {}, Local { led }, init::Monotonics(mono))
    }

    // make sure that no wfi is called
    #[idle()]
    fn idle(_cxt: idle::Context) -> ! {
        rprint!("idle");
        loop {}
    }

    #[task(local = [led])]
    fn toggle_led(ctx: toggle_led::Context) {
        ctx.local.led.toggle().unwrap();
        toggle_led::spawn_after(100.millis()).unwrap();
    }
}
