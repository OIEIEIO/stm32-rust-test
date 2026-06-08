// ================================================================
// File: bemf.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/bemf.rs
// Version: v0.4.6-split-bemf-same-behavior
// Purpose: BEMF pin setup and observe-only floating-phase sampling
//          helpers for the STM32G431CB B-G431B-ESC1 bring-up firmware.
//
// Learning notes:
//   - This module is observe-only. It does not decide commutation timing
//     and it does not close the motor loop.
//   - GPIO_BEMF/PB5 is held as input/high-Z so the BEMF divider remains
//     disabled for the current PWM-off, ground-referenced sampling mode.
//   - The floating phase is selected from the active six-step vector and
//     sampled from ADC2 during the PWM-off window after a blanking delay.
//   - The compact `bemf ...` log line remains the same as the monolithic
//     baseline so hardware behavior and diagnostics stay comparable.
// ================================================================

use core::ptr::{read_volatile, write_volatile};

use rtt_target::rprintln;

use crate::adc::*;
use crate::drive::*;
use crate::gpio::*;
use crate::regs::*;

// GPIO_BEMF (PB5) and the three BEMF sense pins. PB5 is held as an
// input (high-Z) so the on-board divider is DISABLED, which is the
// correct configuration for ground-referenced PWM-off sampling at low
// duty. The sense pins go to analog mode with the analog switch closed.
pub fn setup_bemf_pins() {
    unsafe {
        // GPIO_BEMF = PB5 -> input, no pull (divider disabled).
        set_pin_mode(GPIOB_MODER, GPIO_BEMF_PIN, 0b00);
        let mut pb5_pupdr = read_volatile(GPIOB_PUPDR);
        pb5_pupdr &= !(0b11 << (GPIO_BEMF_PIN * 2));
        write_volatile(GPIOB_PUPDR, pb5_pupdr);

        // BEMF1 = PA4, BEMF2 = PC4, BEMF3 = PB11 -> analog, no pull.
        set_pin_mode(GPIOA_MODER, BEMF1_PIN, 0b11);
        set_pin_mode(GPIOC_MODER, BEMF2_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, BEMF3_PIN, 0b11);

        let mut a_pupdr = read_volatile(GPIOA_PUPDR);
        a_pupdr &= !(0b11 << (BEMF1_PIN * 2));
        write_volatile(GPIOA_PUPDR, a_pupdr);

        let mut c_pupdr = read_volatile(GPIOC_PUPDR);
        c_pupdr &= !(0b11 << (BEMF2_PIN * 2));
        write_volatile(GPIOC_PUPDR, c_pupdr);

        let mut b_pupdr = read_volatile(GPIOB_PUPDR);
        b_pupdr &= !(0b11 << (BEMF3_PIN * 2));
        write_volatile(GPIOB_PUPDR, b_pupdr);

        // Close the analog switch for each sense pin (ASCR).
        let mut a_ascr = read_volatile(GPIOA_ASCR);
        a_ascr |= 1 << BEMF1_PIN;
        write_volatile(GPIOA_ASCR, a_ascr);

        let mut c_ascr = read_volatile(GPIOC_ASCR);
        c_ascr |= 1 << BEMF2_PIN;
        write_volatile(GPIOC_ASCR, c_ascr);

        let mut b_ascr = read_volatile(GPIOB_ASCR);
        b_ascr |= 1 << BEMF3_PIN;
        write_volatile(GPIOB_ASCR, b_ascr);
    }
}

// ------------------------------------------------------------
// BEMF sampling (observe-only)
// ------------------------------------------------------------

// The floating phase is the one not driven this vector; return its
// ADC2 BEMF channel and a label for the log.
pub fn floating_bemf_channel(state: DriveState) -> (u32, &'static str) {
    match state {
        DriveState::VectorUhVl | DriveState::VectorVhUl => (BEMF3_ADC_CHANNEL, "W"),
        DriveState::VectorUhWl | DriveState::VectorWhUl => (BEMF2_ADC_CHANNEL, "V"),
        DriveState::VectorVhWl | DriveState::VectorWhVl => (BEMF1_ADC_CHANNEL, "U"),
        _ => (BEMF1_ADC_CHANNEL, "?"),
    }
}

// Sample the floating-phase BEMF inside the PWM-off window. The high
// side PWMs high while CNT < duty, so the off window is CNT from duty
// up to ARR; we wait for the late part of that window (leaving margin
// for the sample phase to finish before the counter wraps) then read.
pub fn read_bemf_floating(channel: u32, duty: u32) -> u16 {
    unsafe {
        let lo = duty + (TIM1_TEST_ARR - duty) / 2;
        let hi = if TIM1_TEST_ARR > 40 {
            TIM1_TEST_ARR - 40
        } else {
            TIM1_TEST_ARR
        };

        let mut guard: u32 = 0;
        loop {
            let cnt = read_volatile(TIM1_CNT);
            if cnt >= lo && cnt <= hi {
                break;
            }
            guard += 1;
            if guard > 2_000_000 {
                return ADC_TIMEOUT_VALUE;
            }
        }

        adc_select_channel(ADC2_BASE, channel);
        write_volatile(adc_isr(ADC2_BASE), ADC_ISR_EOC | ADC_ISR_EOS);
        let cr = read_volatile(adc_cr(ADC2_BASE));
        write_volatile(adc_cr(ADC2_BASE), cr | ADC_CR_ADSTART);

        for _ in 0..100_000 {
            if (read_volatile(adc_isr(ADC2_BASE)) & ADC_ISR_EOC) != 0 {
                return (read_volatile(adc_dr(ADC2_BASE)) & 0x0FFF) as u16;
            }
        }
    }

    ADC_TIMEOUT_VALUE
}

// One compact per-step line: the four floating-phase BEMF samples plus
// the safety fields. Returns the health gate (af / overlap / temp /
// timeout) — note this still cannot detect a rotor slip.
pub fn log_bemf_step(
    run_id: u32,
    step_index: u32,
    electrical_rev: u32,
    vector_delay: u32,
    duty: u32,
    state: DriveState,
    float_label: &str,
    samples: &[u16; BEMF_SAMPLES_PER_STEP],
    baseline: AdcSnapshot,
) -> u32 {
    let drive = read_drive();
    let temp_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
    let vbus_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

    let timeout = if temp_raw == ADC_TIMEOUT_VALUE
        || vbus_raw == ADC_TIMEOUT_VALUE
        || samples[0] == ADC_TIMEOUT_VALUE
        || samples[1] == ADC_TIMEOUT_VALUE
        || samples[2] == ADC_TIMEOUT_VALUE
        || samples[3] == ADC_TIMEOUT_VALUE
    {
        1
    } else {
        0
    };

    let no_overlap = no_phase_overlap(drive);
    let temp_delta = adc_delta(temp_raw, baseline.temp_raw);
    let temp_ok = if temp_delta < TEMP_DELTA_ABORT_RAW { 1 } else { 0 };

    let health_ok = if drive.af_ok == 1 && no_overlap == 1 && timeout == 0 && temp_ok == 1 {
        1
    } else {
        0
    };

    rprintln!(
        "bemf run={} step={} erev={} delay={} duty={} vector={} float_phase={} b0={} b1={} b2={} b3={} health_ok={} button={} af_ok={} no_phase_overlap={} vbus_raw={} vbus_delta={} temp_raw={} temp_delta={} timeout={}",
        run_id,
        step_index,
        electrical_rev,
        vector_delay,
        duty,
        state.name(),
        float_label,
        samples[0],
        samples[1],
        samples[2],
        samples[3],
        health_ok,
        if button_pressed() { 1 } else { 0 },
        drive.af_ok,
        no_overlap,
        vbus_raw,
        adc_delta(vbus_raw, baseline.vbus_raw),
        temp_raw,
        temp_delta,
        timeout
    );

    health_ok
}
// ================================================================
// End of file: bemf.rs
// Version: v0.4.6-split-bemf-same-behavior
// Generated: 2026-06-07
// Creation date: 2026-06-07
// ================================================================
