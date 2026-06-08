// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.4.9-split-sixstep-same-behavior
// Purpose: STM32G431CB Rust: PWM open-loop six-step with BEMF sense
//          INSTRUMENTATION (observe only; loop stays open)
//          Stage 9 split: register/pin constants moved to regs.rs; generic GPIO helpers moved to gpio.rs; drive state/readback helpers moved to drive.rs; TIM1 helpers moved to tim1.rs; ADC helpers moved to adc.rs; BEMF helpers moved to bemf.rs; delay/dead-man helpers moved to safety.rs; heavy/heartbeat logging helpers moved to log.rs; open-loop six-step ramp runner moved to sixstep.rs
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.3.2:
//   - DRIVE UNCHANGED: same ramp, alignment, dead-man, timer-based
//     commutation. The loop is still OPEN. No commutation decision
//     uses BEMF. The motor spins exactly as v0.3.x did.
//   - Added BEMF sense instrumentation so the floating-phase back-EMF
//     can be OBSERVED before closing the loop:
//       * GPIO_BEMF = PB5 driven as INPUT / high-Z -> divider DISABLED
//         -> PWM-OFF, ground-referenced sampling (correct for low duty).
//       * BEMF1=PA4 (U/OUT1, ADC2_IN17), BEMF2=PC4 (V/OUT2, ADC2_IN5),
//         BEMF3=PB11 (W/OUT3, ADC2_IN14). ADC2 is the BEMF engine.
//       * Each commutation step: blank (demag), then sample the
//         FLOATING phase 4x across the step inside the PWM-OFF window
//         (CNT polled against the duty), logged as b0..b3.
//   - Run log is now per-step "bemf" lines (was per-erev heartbeat),
//     so one electrical rev of log shows the BEMF trajectory.
//   - Speed lowered for legible logging: RAMP_MIN_VECTOR_DELAY
//     50000 -> 150000 (~150 RPM). BEMF smaller than at 450 RPM but
//     clearly present. MAX_VECTOR_STEPS_PER_HOLD 1200 -> 600.
//   - Duty UNCHANGED (20% max).
//   - VERIFY FIRST: confirm each BEMF channel responds to its own
//     phase (sanity-check the PA4/PC4/PB11 -> ADC2 channel numbers
//     against DS12589) before trusting the waveform.
//   - NOTE: still no closed loop; health_ok cannot detect a slip.
//
// Change summary v0.3.1 vs v0.3.0:
//   - PWM frequency raised to ~20 kHz (inaudible). ARR 3999 -> 799 on
//     the default HSI16 clock (16 MHz / 800 = 20 kHz).
//   - Duty constants rescaled to the new ARR to preserve the SAME
//     effective voltages as the v0.3.0 run (10% align/start, 20% max).
//
// Change summary of v0.3.0 vs v0.2.16 (forced-active baseline):
//   - High-side drive: FORCED_ACTIVE (100% on) -> PWM_MODE_1 with duty.
//   - Commutation: direct vector->vector, no per-step bootstrap /
//     deadtime / all-off windows (PWM off-time freewheel recharges
//     the high-side bootstrap caps via the low-side body diode).
//   - Bootstrap precharge: done ONCE at run start (reuses the
//     confirmed Bootstrap*L states), not per commutation.
//   - Alignment: rotor is parked on one vector before the ramp.
//   - Logging: split. Heavy verification dump kept for startup and
//     fault only; a compact heartbeat is emitted once per electrical
//     rev during the run.
//   - Ramp constants retuned for the PWM regime (PWM bounds current,
//     so the start can be faster than the forced-active version).
//
// Unchanged / confirmed-working and reused as-is:
//   GPIO base setup, TIM1 base setup, drive-pin AF mapping, ADC setup,
//   the commutation CCER table (expected_ccer), read_drive,
//   pins_match_state, no_phase_overlap, dead-man release path,
//   ramp_delay_for_step, startup readback, FaultAllOff handling.
//
// PWM frequency note: ARR is 799 on the default HSI16 clock, giving
// ~20 kHz (above hearing). If a PLL has been configured elsewhere the
// frequency scales with the timer clock; recompute ARR if so.
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
mod bemf;
mod safety;
mod log;
mod sixstep;

use crate::drive::*;
use crate::tim1::*;
use crate::adc::*;
use crate::bemf::*;
use crate::safety::*;
use crate::log::*;
use crate::sixstep::*;
use crate::gpio::*;
use crate::regs::*;

// ------------------------------------------------------------
// ADC readback/setup/read helpers moved to adc.rs
// ------------------------------------------------------------

// ------------------------------------------------------------
// Drive state/readback helpers moved to drive.rs
// ------------------------------------------------------------

// ------------------------------------------------------------
// Delay / dead-man helpers moved to safety.rs
// ------------------------------------------------------------

// ------------------------------------------------------------
// TIM1 helpers moved to tim1.rs
// ------------------------------------------------------------

// ------------------------------------------------------------
// ADC helpers moved to adc.rs
// ------------------------------------------------------------

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
// Logging helpers moved to log.rs
// ------------------------------------------------------------

// ------------------------------------------------------------
// Ramp scheduling / open-loop run helpers moved to sixstep.rs
// ------------------------------------------------------------

// ------------------------------------------------------------
// Main
// ------------------------------------------------------------

#[entry]
fn main() -> ! {
    rtt_init_print!();

    setup_gpio_base();
    setup_tim1_base();
    setup_drive_pins_tim1_af();
    setup_bemf_pins();

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
    rprintln!("Version: v0.4.0-bemf-observe-openloop");
    rprintln!("Mode: PWM open-loop six-step + BEMF observe (loop still OPEN)");
    rprintln!("Button released: all outputs off.");
    rprintln!("Button held: precharge -> align hold -> ramped six-step.");
    rprintln!("Release button to stop immediately.");
    rprintln!("No prop. Low voltage. Strict bench current limit.");
    rprintln!("PWM freq = timer_clk / {} (ARR+1). On HSI16 this is ~20 kHz (inaudible).", TIM1_TEST_ARR + 1);
    rprintln!("ADC1 setup status: {}", adc1_setup_status);
    rprintln!("ADC2 setup status: {}", adc2_setup_status);

    rprintln!("BEMF observe (PWM-off sampling, divider disabled):");
    rprintln!("  GPIO_BEMF=PB5 held INPUT/high-Z (divider off, GND ref)");
    rprintln!("  BEMF1=PA4=ADC2_IN17 (U), BEMF2=PC4=ADC2_IN5 (V), BEMF3=PB11=ADC2_IN14 (W)");
    rprintln!("  per step: blank {} then {} samples in PWM-off window", BEMF_BLANK_DELAY, BEMF_SAMPLES_PER_STEP);
    rprintln!("  log line 'bemf ...' shows floating phase b0..b3 per step");
    rprintln!("  VERIFY each phase's BEMF channel responds before trusting data");

    rprintln!("PWM duty (CCR vs ARR={}):", TIM1_TEST_ARR);
    rprintln!("  align duty:      {}", PWM_DUTY_ALIGN);
    rprintln!("  run start duty:  {}", PWM_DUTY_RUN_START);
    rprintln!("  run max duty:    {}", PWM_DUTY_RUN_MAX);
    rprintln!("  inc per erev:    {}", PWM_DUTY_INC_PER_EREV);
    rprintln!("  align hold:      {}", ALIGN_HOLD_DELAY);

    rprintln!("Ramp:");
    rprintln!("  start vector delay: {}", RAMP_START_VECTOR_DELAY);
    rprintln!("  min vector delay:   {}", RAMP_MIN_VECTOR_DELAY);
    rprintln!("  decrement / erev:   {}", RAMP_DECREMENT_PER_ELECTRICAL_REV);
    rprintln!("  max vector steps:   {}", MAX_VECTOR_STEPS_PER_HOLD);

    rprintln!("Six-step sequence (high side PWMs, low side solid on):");
    rprintln!("  1 vector_uh_vl  UH~PWM VL=1");
    rprintln!("  2 vector_uh_wl  UH~PWM WL=1");
    rprintln!("  3 vector_vh_wl  VH~PWM WL=1");
    rprintln!("  4 vector_vh_ul  VH~PWM UL=1");
    rprintln!("  5 vector_wh_ul  WH~PWM UL=1");
    rprintln!("  6 vector_wh_vl  WH~PWM VL=1");

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
    rprintln!("During run: heartbeat once per erev; high pins flicker (PWM).");
    rprintln!("Safety stops: button release, health fault (af/overlap/temp/timeout), max step count.");
    rprintln!("================================================");

    let mut run_id: u32 = 1;

    loop {
        apply_state(DriveState::IdleAllOff);

        if button_pressed() {
            run_pwm_openloop_ramp(run_id, baseline);

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
// Version: v0.4.9-split-sixstep-same-behavior
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================