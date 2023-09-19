//! Periodically reads the voltage of an AFEC0 channel.
#![no_std]
#![no_main]

use panic_rtt_target as _;

#[rtic::app(device = hal::pac, peripherals = true, dispatchers = [UART0])]
mod app {
    use atsamx7x_hal as hal;
    use dwt_systick_monotonic::{DwtSystick, ExtU32};
    use hal::afec::*;
    use hal::clocks::*;
    use hal::efc::*;
    use hal::ehal::adc::OneShot;
    use hal::ehal::digital::v2::ToggleableOutputPin;
    use hal::fugit::RateExtU32;
    use hal::pio::*;
    use rtt_target::{rprintln, rtt_init_print};

    #[monotonic(binds = SysTick, default = true)]
    type Mono = DwtSystick<16_000_000>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        afec: Afec<Afec0>,
        adc_pin: Pin<PB3, Input>,
        pwm_pin: Pin<PA0, Output>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let pac = ctx.device;

        pac.WDT.mr.modify(|_r, c| c.wddis().set_bit());
        pac.RSWDT.mr.modify(|_r, c| c.wddis().set_bit());

        rtt_init_print!();
        rprintln!("reset - cmd_crc_cobs_lib");
        for _ in 0..5 {
            for _ in 0..4000_000 {
                cortex_m::asm::nop();
            }
            rprintln!(".");
        }
        rprintln!("init done");

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

        let banka = hal::pio::BankA::new(pac.PIOA, &mut mck, &slck, BankConfiguration::default());

        let bankb = hal::pio::BankB::new(pac.PIOB, &mut mck, &slck, BankConfiguration::default());

        let mono = DwtSystick::new(
            &mut ctx.core.DCB,
            ctx.core.DWT,
            ctx.core.SYST,
            hclk.systick_freq().to_Hz(),
        );

        let afec = Afec::new_afec0(pac.AFEC0, &mut mck).unwrap();
        let adc_pin = bankb.pb3.into_input(PullDir::PullUp);
        let pwm_pin = banka.pa0.into_output(true);

        adc_sample::spawn().unwrap();

        (
            Shared {},
            Local {
                afec,
                adc_pin,
                pwm_pin,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [afec, pwm_pin, adc_pin])]
    fn adc_sample(ctx: adc_sample::Context) {
        let adc_sample::LocalResources {
            afec,
            adc_pin,
            pwm_pin,
        } = ctx.local;

        // get sample
        let v: f32 = afec.read(adc_pin).unwrap();
        rprintln!("PB3 (channel 2) = {:.2}V", v);

        // toggle
        pwm_pin.toggle().unwrap();

        adc_sample::spawn_after(1.secs()).unwrap();
    }
}
