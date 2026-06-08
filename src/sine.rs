// ================================================================
// File: sine.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/sine.rs
// Version: v0.5.2-openloop-sine-96-fast-loop
// Purpose: Open-loop sine/SPWM runner for the B-G431B-ESC1
//          STM32G431CB Rust motor-control bring-up.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.5.1:
//   - Keeps the 96-step Q10 sine table.
//   - Keeps dead-man-button behavior unchanged.
//   - Keeps the same amplitude/ramp constants from regs.rs.
//   - Moves heavy ADC/TIM1/drive health checks out of every sine step.
//   - Fast loop now writes CCR1/CCR2/CCR3 directly.
//   - Health/readback/logging happens every SINE_HEALTH_EVERY_STEPS.
//   - Goal: higher RPM while preserving the much quieter 96-step waveform.
//
// Behavior:
//   - Button released: all outputs off.
//   - Button held: bootstrap precharge -> fixed sine alignment ->
//     open-loop sine/SPWM electrical-angle ramp.
//   - Release button: all-off.
//   - No pot control yet.
//   - No BEMF decision logic.
//   - No FOC.
//   - No SVPWM.
//   - No closed loop.
//
// Learning notes:
//   - The 96-step table made the motor much quieter and smoother.
//   - The previous per-step health/readback loop dominated timing.
//   - This version separates:
//       fast path: CCR update + button check + delay
//       slow path: ADC/TIM1/drive readback + log
//   - This is still open-loop. The rotor is not measured.
// ================================================================

use core::ptr::write_volatile;

use cortex_m::asm;
use rtt_target::rprintln;

use crate::adc::{adc_delta, read_adc_snapshot, AdcSnapshot};
use crate::drive::{no_phase_overlap, read_drive, DriveState};
use crate::gpio::button_pressed;
use crate::log::log_state;
use crate::regs::*;
use crate::safety::delay_while_button_held;
use crate::tim1::{
    apply_sine_pwm_uvw,
    apply_state,
    enable_sine_pwm_outputs,
    read_tim1_for_sine_pwm,
};

// Heavy health/readback interval.
// With a 96-step table, 960 steps = 10 electrical revolutions.
const SINE_HEALTH_EVERY_STEPS: u32 = 960;

// 96-entry sine table, Q10 scale.
// Full scale = +/-1024.
// Index spacing = 3.75 electrical degrees.
const SINE_Q10: [i16; SINE_TABLE_LEN] = [
    0, 67, 134, 200, 265, 329, 392, 453, 512, 569, 623, 675,
    724, 770, 812, 851, 887, 918, 946, 970, 989, 1004, 1015, 1022,
    1024, 1022, 1015, 1004, 989, 970, 946, 918, 887, 851, 812, 770,
    724, 675, 623, 569, 512, 453, 392, 329, 265, 200, 134, 67,
    0, -67, -134, -200, -265, -329, -392, -453, -512, -569, -623, -675,
    -724, -770, -812, -851, -887, -918, -946, -970, -989, -1004, -1015, -1022,
    -1024, -1022, -1015, -1004, -989, -970, -946, -918, -887, -851, -812, -770,
    -724, -675, -623, -569, -512, -453, -392, -329, -265, -200, -134, -67,
];

fn fast_delay_cycles(cycles: u32) {
    asm::delay(cycles);
}

fn clamp_i32_to_duty(value: i32) -> u32 {
    if value < SINE_PWM_MIN_DUTY as i32 {
        SINE_PWM_MIN_DUTY
    } else if value > SINE_PWM_MAX_DUTY as i32 {
        SINE_PWM_MAX_DUTY
    } else {
        value as u32
    }
}

fn sine_duty(table_index: usize, amplitude: u32) -> u32 {
    let sample = SINE_Q10[table_index] as i32;
    let offset = ((amplitude as i32) * sample) / 1024;
    clamp_i32_to_duty((SINE_PWM_CENTER as i32) + offset)
}

fn sine_duties_for_index(phase_index: usize, amplitude: u32) -> (u32, u32, u32) {
    let u_index = phase_index % SINE_TABLE_LEN;

    // 120 electrical degrees = 32 samples at 96 samples/rev.
    // V is theta - 120 deg, represented as +64 samples.
    // W is theta + 120 deg, represented as +32 samples.
    let v_index = (u_index + ((SINE_TABLE_LEN * 2) / 3)) % SINE_TABLE_LEN;
    let w_index = (u_index + (SINE_TABLE_LEN / 3)) % SINE_TABLE_LEN;

    (
        sine_duty(u_index, amplitude),
        sine_duty(v_index, amplitude),
        sine_duty(w_index, amplitude),
    )
}

fn apply_sine_pwm_uvw_fast(u_duty: u32, v_duty: u32, w_duty: u32) {
    unsafe {
        write_volatile(TIM1_CCR1, u_duty);
        write_volatile(TIM1_CCR2, v_duty);
        write_volatile(TIM1_CCR3, w_duty);
    }
}

fn sine_step_delay_for_step(step_index: u32) -> (u32, u32) {
    let electrical_rev = step_index / (SINE_TABLE_LEN as u32);
    let reduction = electrical_rev * SINE_DECREMENT_PER_ELECTRICAL_REV;

    let delay = if SINE_START_STEP_DELAY > reduction {
        SINE_START_STEP_DELAY - reduction
    } else {
        SINE_MIN_STEP_DELAY
    };

    if delay < SINE_MIN_STEP_DELAY {
        (SINE_MIN_STEP_DELAY, electrical_rev)
    } else {
        (delay, electrical_rev)
    }
}

fn sine_amplitude_for_rev(electrical_rev: u32) -> u32 {
    let amplitude = SINE_PWM_START_AMPLITUDE
        + electrical_rev * SINE_PWM_INC_PER_ELECTRICAL_REV;

    if amplitude > SINE_PWM_RUN_MAX_AMPLITUDE {
        SINE_PWM_RUN_MAX_AMPLITUDE
    } else {
        amplitude
    }
}

fn log_sine_health(
    run_id: u32,
    step_index: u32,
    electrical_rev: u32,
    phase_index: usize,
    step_delay: u32,
    amplitude: u32,
    u_duty: u32,
    v_duty: u32,
    w_duty: u32,
    baseline: AdcSnapshot,
    force_log: bool,
) -> u32 {
    let drive = read_drive();
    let tim1 = read_tim1_for_sine_pwm();
    let adc = read_adc_snapshot();

    let no_overlap = no_phase_overlap(drive);
    let temp_delta = adc_delta(adc.temp_raw, baseline.temp_raw);
    let temp_ok = if temp_delta < TEMP_DELTA_ABORT_RAW { 1 } else { 0 };

    let health_ok = if drive.af_ok == 1
        && no_overlap == 1
        && tim1.tim1_sine_ok == 1
        && adc.timeout == 0
        && temp_ok == 1
    {
        1
    } else {
        0
    };

    if force_log || health_ok != 1 || (step_index % SINE_LOG_EVERY_STEPS) == 0 {
        rprintln!(
            "sine run={} step={} erev={} phase_idx={} delay={} amp={} u={} v={} w={} health_ok={} button={} af_ok={} no_phase_overlap={} tim1_sine_ok={} ccer_ok={} pwm_modes_ok={} moe={} deadtime={} ccr1={} ccr2={} ccr3={} UH={} UL={} VH={} VL={} WH={} WL={} vbus_raw={} vbus_delta={} temp_raw={} temp_delta={} temp_ok={} pot_raw={} timeout={}",
            run_id,
            step_index,
            electrical_rev,
            phase_index,
            step_delay,
            amplitude,
            u_duty,
            v_duty,
            w_duty,
            health_ok,
            if button_pressed() { 1 } else { 0 },
            drive.af_ok,
            no_overlap,
            tim1.tim1_sine_ok,
            tim1.ccer_ok,
            tim1.pwm_modes_ok,
            tim1.moe,
            tim1.deadtime,
            tim1.ccr1,
            tim1.ccr2,
            tim1.ccr3,
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
            adc.pot_raw,
            adc.timeout
        );
    }

    health_ok
}

pub fn run_sine_openloop(run_id: u32, baseline: AdcSnapshot) {
    rprintln!(
        "sine_run_start run={} hold_button_to_run release_to_stop center={} min={} max={} start_amp={} max_amp={} amp_inc_per_erev={} table_len={} align_hold={} start_delay={} min_delay={} dec_per_erev={} max_steps={} log_every={} health_every={}",
        run_id,
        SINE_PWM_CENTER,
        SINE_PWM_MIN_DUTY,
        SINE_PWM_MAX_DUTY,
        SINE_PWM_START_AMPLITUDE,
        SINE_PWM_RUN_MAX_AMPLITUDE,
        SINE_PWM_INC_PER_ELECTRICAL_REV,
        SINE_TABLE_LEN,
        SINE_ALIGN_HOLD_DELAY,
        SINE_START_STEP_DELAY,
        SINE_MIN_STEP_DELAY,
        SINE_DECREMENT_PER_ELECTRICAL_REV,
        SINE_MAX_STEPS_PER_HOLD,
        SINE_LOG_EVERY_STEPS,
        SINE_HEALTH_EVERY_STEPS
    );

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

    let (u_align, v_align, w_align) =
        sine_duties_for_index(0, SINE_PWM_START_AMPLITUDE);

    apply_sine_pwm_uvw(u_align, v_align, w_align);
    enable_sine_pwm_outputs();

    let align_health = log_sine_health(
        run_id,
        0,
        0,
        0,
        SINE_ALIGN_HOLD_DELAY,
        SINE_PWM_START_AMPLITUDE,
        u_align,
        v_align,
        w_align,
        baseline,
        true,
    );

    if align_health != 1 {
        apply_state(DriveState::FaultAllOff);
        log_state(run_id, 0, 0, 0, DriveState::FaultAllOff, baseline);
        return;
    }

    if !delay_while_button_held(SINE_ALIGN_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }

    let mut sine_steps: u32 = 0;
    let mut cycle_ok: u32 = 1;
    let mut last_step_delay: u32 = SINE_START_STEP_DELAY;
    let mut last_electrical_rev: u32 = 0;
    let mut last_phase_index: usize = 0;
    let mut last_amplitude: u32 = SINE_PWM_START_AMPLITUDE;
    let mut last_u: u32 = u_align;
    let mut last_v: u32 = v_align;
    let mut last_w: u32 = w_align;

    while button_pressed() && sine_steps < SINE_MAX_STEPS_PER_HOLD {
        let phase_index = (sine_steps % (SINE_TABLE_LEN as u32)) as usize;
        let (step_delay, electrical_rev) = sine_step_delay_for_step(sine_steps);
        let amplitude = sine_amplitude_for_rev(electrical_rev);

        let (u_duty, v_duty, w_duty) =
            sine_duties_for_index(phase_index, amplitude);

        apply_sine_pwm_uvw_fast(u_duty, v_duty, w_duty);

        last_step_delay = step_delay;
        last_electrical_rev = electrical_rev;
        last_phase_index = phase_index;
        last_amplitude = amplitude;
        last_u = u_duty;
        last_v = v_duty;
        last_w = w_duty;

        fast_delay_cycles(step_delay);

        sine_steps += 1;

        if !button_pressed() {
            break;
        }

        if (sine_steps % SINE_HEALTH_EVERY_STEPS) == 0 {
            let health_ok = log_sine_health(
                run_id,
                sine_steps,
                electrical_rev,
                phase_index,
                step_delay,
                amplitude,
                u_duty,
                v_duty,
                w_duty,
                baseline,
                false,
            );

            if health_ok != 1 {
                cycle_ok = 0;
                break;
            }
        }
    }

    apply_state(DriveState::IdleAllOff);

    let stop_reason = if cycle_ok == 0 {
        "health_fault"
    } else if sine_steps >= SINE_MAX_STEPS_PER_HOLD {
        "max_steps"
    } else {
        "button_released"
    };

    rprintln!(
        "sine_run_stop run={} cycle_ok={} sine_steps={} button={} reason={} last_erev={} last_phase_idx={} last_delay={} last_amp={} last_u={} last_v={} last_w={}",
        run_id,
        cycle_ok,
        sine_steps,
        if button_pressed() { 1 } else { 0 },
        stop_reason,
        last_electrical_rev,
        last_phase_index,
        last_step_delay,
        last_amplitude,
        last_u,
        last_v,
        last_w
    );

    if cycle_ok == 0 {
        apply_state(DriveState::FaultAllOff);
        log_state(run_id, 9998, 0, 0, DriveState::FaultAllOff, baseline);
    }

    log_state(run_id, 9999, 0, 0, DriveState::IdleAllOff, baseline);
}

// ================================================================
// Footer
// File: sine.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/sine.rs
// Version: v0.5.2-openloop-sine-96-fast-loop
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================