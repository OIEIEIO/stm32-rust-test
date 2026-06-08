// ================================================================
// File: log.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/log.rs
// Version: v0.4.8-split-log-same-behavior
// Purpose: Runtime verification and heartbeat logging helpers for
//          the B-G431B-ESC1 Rust open-loop six-step + BEMF observe
//          firmware.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Split note:
//   This file is a behavior-preserving extraction from main.rs.
//   It owns the heavy startup/fault verification log and the compact
//   per-electrical-rev heartbeat log. It does not change drive state,
//   PWM duty, ADC sampling, BEMF sampling, or dead-man behavior.
//
// Bring-up lesson:
//   During PWM operation, instantaneous high-side GPIO readback is not
//   a stable safety truth because the high side is switching. The run
//   heartbeat therefore treats high-side pin values as reported data,
//   while the health gate uses deterministic checks: alternate-function
//   mapping, phase-overlap absence, ADC timeout, and temperature delta.
// ================================================================

use rtt_target::rprintln;

use crate::adc::*;
use crate::drive::*;
use crate::gpio::button_pressed;
use crate::regs::*;
use crate::safety::delay_cycles;
use crate::tim1::read_tim1_for_state;

// ------------------------------------------------------------
// Heavy verification log (startup / idle / fault only)
// ------------------------------------------------------------

pub fn log_state(
    run_id: u32,
    step_index: u32,
    electrical_rev: u32,
    vector_delay: u32,
    state: DriveState,
    baseline: AdcSnapshot,
) -> u32 {
    delay_cycles(STATE_SETTLE_DELAY);

    let drive = read_drive();
    let tim1 = read_tim1_for_state(state);
    let adc = read_adc_snapshot();

    let pins_ok = pins_match_state(drive, state);
    let no_overlap = no_phase_overlap(drive);
    let active_count = active_pin_count(drive);

    let expected = state.expected_pins();
    let expected_active_count =
        expected.0 + expected.1 + expected.2 + expected.3 + expected.4 + expected.5;

    let active_count_ok = if active_count == expected_active_count { 1 } else { 0 };
    let temp_delta = adc_delta(adc.temp_raw, baseline.temp_raw);
    let temp_ok = if temp_delta < TEMP_DELTA_ABORT_RAW { 1 } else { 0 };

    let state_ok = if drive.af_ok == 1
        && pins_ok == 1
        && no_overlap == 1
        && active_count_ok == 1
        && tim1.tim1_basic_ok == 1
        && adc.timeout == 0
        && temp_ok == 1
    {
        1
    } else {
        0
    };

    rprintln!(
        "run={} step={} erev={} delay={} state={} state_ok={} button={} af_ok={} pins_ok={} no_phase_overlap={} active_count={} active_count_ok={} tim1_ok={} expected_ccer={} tim1_ccer={} tim1_moe={} forced_modes_ok={} UH={} UL={} VH={} VL={} WH={} WL={} cnt_a={} cnt_b={} ccmr1={} ccmr2={} bdtr={} pot_raw={} temp_raw={} temp_delta={} temp_ok={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} timeout={}",
        run_id,
        step_index,
        electrical_rev,
        vector_delay,
        state.name(),
        state_ok,
        if button_pressed() { 1 } else { 0 },
        drive.af_ok,
        pins_ok,
        no_overlap,
        active_count,
        active_count_ok,
        tim1.tim1_basic_ok,
        state.expected_ccer(),
        tim1.ccer,
        tim1.moe,
        tim1.forced_modes_ok,
        drive.uh_pin,
        drive.ul_pin,
        drive.vh_pin,
        drive.vl_pin,
        drive.wh_pin,
        drive.wl_pin,
        tim1.cnt_a,
        tim1.cnt_b,
        tim1.ccmr1,
        tim1.ccmr2,
        tim1.bdtr,
        adc.pot_raw,
        adc.temp_raw,
        temp_delta,
        temp_ok,
        adc.vbus_raw,
        adc_delta(adc.vbus_raw, baseline.vbus_raw),
        adc.op1_raw,
        adc_delta(adc.op1_raw, baseline.op1_raw),
        adc.op2_raw,
        adc_delta(adc.op2_raw, baseline.op2_raw),
        adc.op3_raw,
        adc_delta(adc.op3_raw, baseline.op3_raw),
        adc.timeout
    );

    state_ok
}

// ------------------------------------------------------------
// Compact run heartbeat (once per electrical rev during the ramp)
// ------------------------------------------------------------
// High-side pins are PWMing during a vector, so an instantaneous IDR
// read of a high pin is non-deterministic and is reported, not
// asserted. The health gate uses only deterministic / safety signals:
// AF mapping, no high+low overlap, temp delta, ADC timeout.

pub fn log_heartbeat(
    run_id: u32,
    step_index: u32,
    electrical_rev: u32,
    vector_delay: u32,
    duty: u32,
    state: DriveState,
    baseline: AdcSnapshot,
) -> u32 {
    let drive = read_drive();
    let adc = read_adc_snapshot();

    let no_overlap = no_phase_overlap(drive);
    let temp_delta = adc_delta(adc.temp_raw, baseline.temp_raw);
    let temp_ok = if temp_delta < TEMP_DELTA_ABORT_RAW { 1 } else { 0 };

    let health_ok = if drive.af_ok == 1
        && no_overlap == 1
        && adc.timeout == 0
        && temp_ok == 1
    {
        1
    } else {
        0
    };

    rprintln!(
        "hb run={} step={} erev={} delay={} duty={} vector={} health_ok={} button={} af_ok={} no_phase_overlap={} UH={} UL={} VH={} VL={} WH={} WL={} vbus_raw={} vbus_delta={} temp_raw={} temp_delta={} temp_ok={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} timeout={}",
        run_id,
        step_index,
        electrical_rev,
        vector_delay,
        duty,
        state.name(),
        health_ok,
        if button_pressed() { 1 } else { 0 },
        drive.af_ok,
        no_overlap,
        drive.uh_pin,
        drive.ul_pin,
        drive.vh_pin,
        drive.vl_pin,
        drive.wh_pin,
        drive.wl_pin,
        adc.vbus_raw,
        adc_delta(adc.vbus_raw, baseline.vbus_raw),
        adc.temp_raw,
        temp_delta,
        temp_ok,
        adc.op1_raw,
        adc_delta(adc.op1_raw, baseline.op1_raw),
        adc.op2_raw,
        adc_delta(adc.op2_raw, baseline.op2_raw),
        adc.op3_raw,
        adc_delta(adc.op3_raw, baseline.op3_raw),
        adc.timeout
    );

    health_ok
}


// ================================================================
// Footer
// File: log.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/log.rs
// Version: v0.4.8-split-log-same-behavior
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================
