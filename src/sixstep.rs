// ================================================================
// File: sixstep.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/sixstep.rs
// Version: v0.4.9-split-sixstep-same-behavior
// Purpose: Open-loop six-step ramp runner for the B-G431B-ESC1 Rust
//          PWM + BEMF-observe bring-up firmware.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Split note:
//   This file is a behavior-preserving extraction from main.rs.
//   It owns only the existing open-loop sequence timing and run loop:
//   bootstrap precharge, rotor alignment, ramped six-step commutation,
//   BEMF observe-only sampling, health logging, and dead-man exits.
//
// Bring-up lesson:
//   BEMF is still observe-only here. Nothing in this file uses BEMF to
//   decide commutation timing. The motor-control behavior remains the
//   same open-loop ramp from the monolithic v0.4.0 baseline.
// ================================================================

use rtt_target::rprintln;

use crate::adc::AdcSnapshot;
use crate::bemf::{floating_bemf_channel, log_bemf_step, read_bemf_floating};
use crate::drive::DriveState;
use crate::gpio::button_pressed;
use crate::log::{log_heartbeat, log_state};
use crate::regs::*;
use crate::safety::delay_while_button_held;
use crate::tim1::{apply_pwm_vector, apply_state};

// ------------------------------------------------------------
// Ramp scheduling (UNCHANGED)
// ------------------------------------------------------------

pub fn ramp_delay_for_step(vector_step: u32) -> (u32, u32) {
    let electrical_rev = vector_step / 6;
    let reduction = electrical_rev * RAMP_DECREMENT_PER_ELECTRICAL_REV;

    let delay = if RAMP_START_VECTOR_DELAY > reduction {
        RAMP_START_VECTOR_DELAY - reduction
    } else {
        RAMP_MIN_VECTOR_DELAY
    };

    if delay < RAMP_MIN_VECTOR_DELAY {
        (RAMP_MIN_VECTOR_DELAY, electrical_rev)
    } else {
        (delay, electrical_rev)
    }
}

// ------------------------------------------------------------
// PWM open-loop run: precharge -> align -> ramp
// ------------------------------------------------------------

pub fn run_pwm_openloop_ramp(run_id: u32, baseline: AdcSnapshot) {
    let sequence = [
        DriveState::VectorUhVl,
        DriveState::VectorUhWl,
        DriveState::VectorVhWl,
        DriveState::VectorVhUl,
        DriveState::VectorWhUl,
        DriveState::VectorWhVl,
    ];

    rprintln!(
        "pwm_run_start run={} hold_button_to_run release_to_stop align_duty={} run_start_duty={} run_max_duty={} inc_per_erev={} align_hold={} start_delay={} min_delay={} dec_per_erev={} max_steps={}",
        run_id,
        PWM_DUTY_ALIGN,
        PWM_DUTY_RUN_START,
        PWM_DUTY_RUN_MAX,
        PWM_DUTY_INC_PER_EREV,
        ALIGN_HOLD_DELAY,
        RAMP_START_VECTOR_DELAY,
        RAMP_MIN_VECTOR_DELAY,
        RAMP_DECREMENT_PER_ELECTRICAL_REV,
        MAX_VECTOR_STEPS_PER_HOLD
    );

    // One-time bootstrap precharge: turn each phase low FET on briefly
    // so all three high-side bootstrap caps are charged before the
    // first PWM vector. Reuses the confirmed Bootstrap*L states.
    apply_state(DriveState::BootstrapUL);
    if !delay_while_button_held(BOOTSTRAP_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }
    apply_state(DriveState::BootstrapVL);
    if !delay_while_button_held(BOOTSTRAP_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }
    apply_state(DriveState::BootstrapWL);
    if !delay_while_button_held(BOOTSTRAP_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }

    // Alignment: park the rotor on the first vector at align duty.
    apply_pwm_vector(sequence[0], PWM_DUTY_ALIGN);
    let align_health = log_heartbeat(
        run_id,
        0,
        0,
        ALIGN_HOLD_DELAY,
        PWM_DUTY_ALIGN,
        sequence[0],
        baseline,
    );
    if align_health != 1 {
        apply_state(DriveState::FaultAllOff);
        log_state(run_id, 0, 0, 0, DriveState::FaultAllOff, baseline);
        return;
    }
    if !delay_while_button_held(ALIGN_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }

    // Ramp: direct vector-to-vector PWM commutation, no dead windows.
    let mut vector_steps: u32 = 0;
    let mut cycle_ok: u32 = 1;

    while button_pressed() && vector_steps < MAX_VECTOR_STEPS_PER_HOLD {
        let index = (vector_steps % 6) as usize;
        let vector_state = sequence[index];
        let (vector_delay, electrical_rev) = ramp_delay_for_step(vector_steps);

        let duty_unclamped = PWM_DUTY_RUN_START + electrical_rev * PWM_DUTY_INC_PER_EREV;
        let duty = if duty_unclamped > PWM_DUTY_RUN_MAX {
            PWM_DUTY_RUN_MAX
        } else {
            duty_unclamped
        };

        apply_pwm_vector(vector_state, duty);

        let (bemf_chan, float_label) = floating_bemf_channel(vector_state);
        let mut samples: [u16; BEMF_SAMPLES_PER_STEP] =
            [ADC_TIMEOUT_VALUE; BEMF_SAMPLES_PER_STEP];

        // Post-commutation blanking (demag) before the BEMF is valid.
        if !delay_while_button_held(BEMF_BLANK_DELAY) {
            apply_state(DriveState::IdleAllOff);
            rprintln!(
                "pwm_run_stop run={} cycle_ok=1 vector_steps={} reason=button_released_mid_step",
                run_id,
                vector_steps
            );
            log_state(run_id, 9999, 0, 0, DriveState::IdleAllOff, baseline);
            return;
        }

        // Sample the floating phase across the rest of the step, each
        // sample taken inside the PWM-off window. Total hold ~= vector_delay.
        let remaining = if vector_delay > BEMF_BLANK_DELAY {
            vector_delay - BEMF_BLANK_DELAY
        } else {
            0
        };
        let sub = remaining / (BEMF_SAMPLES_PER_STEP as u32);

        let mut released = false;
        let mut i = 0usize;
        while i < BEMF_SAMPLES_PER_STEP {
            if sub > 0 && !delay_while_button_held(sub) {
                released = true;
                break;
            }
            samples[i] = read_bemf_floating(bemf_chan, duty);
            i += 1;
        }

        if released {
            apply_state(DriveState::IdleAllOff);
            rprintln!(
                "pwm_run_stop run={} cycle_ok=1 vector_steps={} reason=button_released_mid_step",
                run_id,
                vector_steps
            );
            log_state(run_id, 9999, 0, 0, DriveState::IdleAllOff, baseline);
            return;
        }

        let health_ok = log_bemf_step(
            run_id,
            vector_steps + 1,
            electrical_rev,
            vector_delay,
            duty,
            vector_state,
            float_label,
            &samples,
            baseline,
        );

        if health_ok != 1 {
            cycle_ok = 0;
            break;
        }

        vector_steps += 1;
    }

    apply_state(DriveState::IdleAllOff);

    rprintln!(
        "pwm_run_stop run={} cycle_ok={} vector_steps={} button={} reason={}",
        run_id,
        cycle_ok,
        vector_steps,
        if button_pressed() { 1 } else { 0 },
        if cycle_ok == 0 {
            "health_fault"
        } else if vector_steps >= MAX_VECTOR_STEPS_PER_HOLD {
            "max_steps"
        } else {
            "button_released"
        }
    );

    if cycle_ok == 0 {
        apply_state(DriveState::FaultAllOff);
        log_state(run_id, 9998, 0, 0, DriveState::FaultAllOff, baseline);
    }

    log_state(run_id, 9999, 0, 0, DriveState::IdleAllOff, baseline);
}
// ================================================================
// Footer
// File: sixstep.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/sixstep.rs
// Version: v0.4.9-split-sixstep-same-behavior
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================
