// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.5.0-openloop-sine-spwm
// Purpose: STM32G431CB Rust: first open-loop sine/SPWM motor test for
//          the ST B-G431B-ESC1 board.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.4.1 split baseline:
//   - Keeps the split module structure.
//   - Keeps the known-good six-step and BEMF files in the repo.
//   - Runtime test path is switched from six-step open-loop ramp to
//     sine/SPWM open-loop ramp.
//   - User experience is intentionally simple:
//       button released -> all outputs off
//       button held     -> precharge -> sine alignment -> sine ramp
//       button released -> immediate all-off
//   - No potentiometer control yet.
//   - No BEMF decision logic.
//   - No closed-loop commutation.
//   - No FOC.
//   - No SVPWM.
//
// Learning notes:
//   - Sine/SPWM mode drives all three phases using TIM1 CH1/CH1N,
//     CH2/CH2N, and CH3/CH3N complementary PWM.
//   - This differs from six-step mode, where one phase floats and BEMF
//     can be observed. In sine/SPWM, BEMF observe is not meaningful
//     because no phase is intentionally floating.
//   - The first test remains dead-man-button controlled and conservative:
//     fixed amplitude ramp, fixed electrical-angle ramp, low voltage,
//     strict current limit, no prop.
// ================================================================

#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

mod regs;
mod gpio;
mod drive;
mod tim1;
mod adc;
#[allow(dead_code)]
mod bemf;
mod safety;
mod log;
#[allow(dead_code)]
mod sixstep;
mod sine;

use crate::adc::*;
use crate::drive::*;
use crate::gpio::*;
use crate::log::*;
use crate::regs::*;
use crate::safety::*;
use crate::sine::*;
use crate::tim1::*;

// ------------------------------------------------------------
// Peripheral setup
// ------------------------------------------------------------

fn setup_gpio_base() {
    unsafe {
        let rcc_ahb2enr = read_volatile(RCC_AHB2ENR);
        write_volatile(
            RCC_AHB2ENR,
            rcc_ahb2enr
                | RCC_AHB2ENR_GPIOAEN
                | RCC_AHB2ENR_GPIOBEN
                | RCC_AHB2ENR_GPIOCEN
                | RCC_AHB2ENR_ADC12EN,
        );

        delay_cycles(8_000);

        force_drive_output_latches_low();

        set_pin_mode(GPIOA_MODER, VBUS_PIN, 0b11);
        set_pin_mode(GPIOA_MODER, OP1_OUT_PIN, 0b11);
        set_pin_mode(GPIOA_MODER, OP2_OUT_PIN, 0b11);

        let mut gpioa_pupdr = read_volatile(GPIOA_PUPDR);
        gpioa_pupdr &= !(0b11 << (VBUS_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP1_OUT_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP2_OUT_PIN * 2));
        write_volatile(GPIOA_PUPDR, gpioa_pupdr);

        let mut gpioa_ascr = read_volatile(GPIOA_ASCR);
        gpioa_ascr |= 1 << VBUS_PIN;
        gpioa_ascr |= 1 << OP1_OUT_PIN;
        gpioa_ascr |= 1 << OP2_OUT_PIN;
        write_volatile(GPIOA_ASCR, gpioa_ascr);

        set_pin_mode(GPIOB_MODER, OP3_OUT_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, POT_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, TEMP_PIN, 0b11);

        let mut gpiob_pupdr = read_volatile(GPIOB_PUPDR);
        gpiob_pupdr &= !(0b11 << (OP3_OUT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (POT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (TEMP_PIN * 2));
        write_volatile(GPIOB_PUPDR, gpiob_pupdr);

        let mut gpiob_ascr = read_volatile(GPIOB_ASCR);
        gpiob_ascr |= 1 << OP3_OUT_PIN;
        gpiob_ascr |= 1 << POT_PIN;
        gpiob_ascr |= 1 << TEMP_PIN;
        write_volatile(GPIOB_ASCR, gpiob_ascr);

        set_pin_mode(GPIOC_MODER, STATUS_LED_PIN, 0b01);

        let mut gpioc_otyper = read_volatile(GPIOC_OTYPER);
        gpioc_otyper &= !(1 << STATUS_LED_PIN);
        write_volatile(GPIOC_OTYPER, gpioc_otyper);

        set_pin_mode(GPIOC_MODER, USER_BUTTON_PIN, 0b00);

        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (STATUS_LED_PIN * 2));
        gpioc_pupdr &= !(0b11 << (USER_BUTTON_PIN * 2));
        gpioc_pupdr |= 0b01 << (USER_BUTTON_PIN * 2);
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);

        led_low();
    }
}

fn setup_drive_pins_tim1_af() {
    unsafe {
        write_volatile(TIM1_CCER, 0);

        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr &= !TIM1_BDTR_MOE;
        bdtr |= TIM1_BDTR_OSSI | TIM1_BDTR_OSSR;
        write_volatile(TIM1_BDTR, bdtr);

        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        force_drive_output_latches_low();

        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_UH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_WH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VL_PIN, AF_TIM1_PA);

        set_pin_mode(GPIOA_MODER, DRIVE_UH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_VH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_WH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_VL_PIN, 0b10);

        let mut gpioa_otyper = read_volatile(GPIOA_OTYPER);
        gpioa_otyper &= !(1 << DRIVE_UH_PIN);
        gpioa_otyper &= !(1 << DRIVE_VH_PIN);
        gpioa_otyper &= !(1 << DRIVE_WH_PIN);
        gpioa_otyper &= !(1 << DRIVE_VL_PIN);
        write_volatile(GPIOA_OTYPER, gpioa_otyper);

        let mut gpioa_pupdr = read_volatile(GPIOA_PUPDR);
        gpioa_pupdr &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_VL_PIN * 2));
        write_volatile(GPIOA_PUPDR, gpioa_pupdr);

        let mut gpioa_ospeedr = read_volatile(GPIOA_OSPEEDR);
        gpioa_ospeedr &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_VL_PIN * 2));
        write_volatile(GPIOA_OSPEEDR, gpioa_ospeedr);

        set_pin_af(GPIOB_AFRL, GPIOB_AFRH, DRIVE_WL_PIN, AF_TIM1_N);
        set_pin_mode(GPIOB_MODER, DRIVE_WL_PIN, 0b10);

        let mut gpiob_otyper = read_volatile(GPIOB_OTYPER);
        gpiob_otyper &= !(1 << DRIVE_WL_PIN);
        write_volatile(GPIOB_OTYPER, gpiob_otyper);

        let mut gpiob_pupdr = read_volatile(GPIOB_PUPDR);
        gpiob_pupdr &= !(0b11 << (DRIVE_WL_PIN * 2));
        write_volatile(GPIOB_PUPDR, gpiob_pupdr);

        let mut gpiob_ospeedr = read_volatile(GPIOB_OSPEEDR);
        gpiob_ospeedr &= !(0b11 << (DRIVE_WL_PIN * 2));
        write_volatile(GPIOB_OSPEEDR, gpiob_ospeedr);

        set_pin_af(GPIOC_AFRL, GPIOC_AFRH, DRIVE_UL_PIN, AF_TIM1_N);
        set_pin_mode(GPIOC_MODER, DRIVE_UL_PIN, 0b10);

        let mut gpioc_otyper = read_volatile(GPIOC_OTYPER);
        gpioc_otyper &= !(1 << DRIVE_UL_PIN);
        write_volatile(GPIOC_OTYPER, gpioc_otyper);

        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (DRIVE_UL_PIN * 2));
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);

        let mut gpioc_ospeedr = read_volatile(GPIOC_OSPEEDR);
        gpioc_ospeedr &= !(0b11 << (DRIVE_UL_PIN * 2));
        write_volatile(GPIOC_OSPEEDR, gpioc_ospeedr);

        apply_state(DriveState::IdleAllOff);
    }
}

// ------------------------------------------------------------
// Main
// ------------------------------------------------------------

#[entry]
fn main() -> ! {
    rtt_init_print!();

    setup_gpio_base();
    setup_tim1_base();
    setup_drive_pins_tim1_af();

    apply_state(DriveState::IdleAllOff);

    let (adc1_setup_status, adc2_setup_status) = setup_adc_for_board_monitor();

    let baseline = read_adc_snapshot();
    let startup_drive = read_drive();
    let startup_tim1 = read_tim1_for_state(DriveState::IdleAllOff);

    let startup_pins_ok = pins_match_state(startup_drive, DriveState::IdleAllOff);

    let startup_ok = if startup_drive.af_ok == 1
        && startup_pins_ok == 1
        && no_phase_overlap(startup_drive) == 1
        && startup_tim1.tim1_basic_ok == 1
        && baseline.timeout == 0
    {
        1
    } else {
        0
    };

    rprintln!("================================================");
    rprintln!("B-G431B-ESC1 Rust bring-up");
    rprintln!("Version: v0.5.0-openloop-sine-spwm");
    rprintln!("Mode: open-loop sine/SPWM test");
    rprintln!("Button released: all outputs off.");
    rprintln!("Button held: precharge -> sine align -> open-loop sine ramp.");
    rprintln!("Release button to stop immediately.");
    rprintln!("No prop. Low voltage. Strict bench current limit.");
    rprintln!(
        "PWM freq = timer_clk / {} (ARR+1). On HSI16 this is ~20 kHz.",
        TIM1_TEST_ARR + 1
    );
    rprintln!("ADC1 setup status: {}", adc1_setup_status);
    rprintln!("ADC2 setup status: {}", adc2_setup_status);

    rprintln!("Sine/SPWM:");
    rprintln!("  TIM1 complementary outputs: CH1/CH1N CH2/CH2N CH3/CH3N");
    rprintln!("  center CCR:      {}", SINE_PWM_CENTER);
    rprintln!("  min CCR clamp:   {}", SINE_PWM_MIN_DUTY);
    rprintln!("  max CCR clamp:   {}", SINE_PWM_MAX_DUTY);
    rprintln!("  start amplitude: {}", SINE_PWM_START_AMPLITUDE);
    rprintln!("  max amplitude:   {}", SINE_PWM_RUN_MAX_AMPLITUDE);
    rprintln!("  amp inc / erev:  {}", SINE_PWM_INC_PER_ELECTRICAL_REV);
    rprintln!("  table length:    {}", SINE_TABLE_LEN);
    rprintln!("  deadtime DTG:    {}", TIM1_BDTR_SINE_SAFE_DTG);

    rprintln!("Sine ramp:");
    rprintln!("  align hold:       {}", SINE_ALIGN_HOLD_DELAY);
    rprintln!("  start step delay: {}", SINE_START_STEP_DELAY);
    rprintln!("  min step delay:   {}", SINE_MIN_STEP_DELAY);
    rprintln!("  decrement / erev: {}", SINE_DECREMENT_PER_ELECTRICAL_REV);
    rprintln!("  max sine steps:   {}", SINE_MAX_STEPS_PER_HOLD);
    rprintln!("  log every steps:  {}", SINE_LOG_EVERY_STEPS);

    rprintln!("Important:");
    rprintln!("  BEMF observe is not used in sine/SPWM mode.");
    rprintln!("  Potentiometer is read only; it does not control this first test.");
    rprintln!("  This is open-loop angle generation only.");

    rprintln!(
        "startup: startup_ok={} af_ok={} pins_ok={} no_phase_overlap={} tim1_ok={} UH={} UL={} VH={} VL={} WH={} WL={} ccer={} moe={} forced_modes_ok={} pot_raw={} temp_raw={} vbus_raw={} op1_raw={} op2_raw={} op3_raw={} timeout={}",
        startup_ok,
        startup_drive.af_ok,
        startup_pins_ok,
        no_phase_overlap(startup_drive),
        startup_tim1.tim1_basic_ok,
        startup_drive.uh_pin,
        startup_drive.ul_pin,
        startup_drive.vh_pin,
        startup_drive.vl_pin,
        startup_drive.wh_pin,
        startup_drive.wl_pin,
        startup_tim1.ccer,
        startup_tim1.moe,
        startup_tim1.forced_modes_ok,
        baseline.pot_raw,
        baseline.temp_raw,
        baseline.vbus_raw,
        baseline.op1_raw,
        baseline.op2_raw,
        baseline.op3_raw,
        baseline.timeout
    );

    rprintln!("Expected idle: startup_ok=1 UH=0 UL=0 VH=0 VL=0 WH=0 WL=0");
    rprintln!("During run: sine log lines show CCR1/CCR2/CCR3 and health gate.");
    rprintln!("Safety stops: button release, health fault, max step count.");
    rprintln!("================================================");

    let mut run_id: u32 = 1;

    loop {
        apply_state(DriveState::IdleAllOff);

        if button_pressed() {
            run_sine_openloop(run_id, baseline);

            while button_pressed() {
                apply_state(DriveState::IdleAllOff);
                delay_cycles(RELEASE_CHECK_CHUNK);
            }

            run_id = run_id.wrapping_add(1);
        } else {
            led_high();
            delay_cycles(IDLE_LED_ON_DELAY);
            led_low();

            log_state(run_id, 0, 0, 0, DriveState::IdleAllOff, baseline);

            delay_cycles(IDLE_LED_OFF_DELAY);
            delay_cycles(IDLE_LOG_DELAY);
        }
    }
}

// ================================================================
// Footer
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.5.0-openloop-sine-spwm
// Created: 2026-06-07
// Generated timestamp: 2026-06-08T02:09:30Z
// ================================================================