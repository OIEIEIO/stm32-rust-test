// ================================================================
// File: sine.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/sine.rs
// Version: v0.5.25-injected-ab-op2-op3v
// Purpose: Open-loop sine/SPWM runner with TIM1_CH4-triggered injected
//          ADC A/B diagnostics:
//            A = ADC1 OP1 + ADC2 OP2
//            B = ADC1 OP1 + ADC2 OP3/VOPAMP3
//          plus OP3/VOPAMP3 regular ADC comparison and compact sector
//          summaries for the B-G431B-ESC1 STM32G431CB bring-up.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs first v0.5.25 draft:
//   - Reads/logs all FocPrepSample fields so cargo build is warning-free.
//   - Adds a_jeoc1/a_jeoc2/b_jeoc1/b_jeoc2 to focmap_ab.
//   - Adds primary_ok to focmap_ab and sine heartbeat.
//   - No motor-drive, ADC, TIM1, OPAMP, ramp, or safety behavior change.
//
// Change summary vs v0.5.24:
//   - Adds injected-channel A/B diagnostic log fields.
//   - Keeps motor drive, sine table, ramp, and safety behavior unchanged.
//   - A pair proves the existing OP1/OP2 injected path remains healthy.
//   - B pair tests ADC2 injected OP3/VOPAMP3 internal routing.
//   - OP3/VOPAMP3 regular read remains logged for comparison.
//   - Diagnostic only: no current control and no FOC math.
//
// Behavior:
//   - Button released: all outputs off.
//   - Button held: bootstrap precharge -> fixed sine alignment ->
//     open-loop sine/SPWM electrical-angle ramp.
//   - Release button: all-off.
// ================================================================

use core::ptr::write_volatile;

use cortex_m::asm;
use rtt_target::rprintln;

use crate::adc::{adc_delta, read_adc_snapshot, AdcSnapshot};
use crate::current_sense::{
    read_foc_prep_sample,
    signed_count_abs,
    signed_count_diff,
    signed_count_sign,
    update_foc_sector_summary,
    CurrentSenseOffsets,
    FocSectorSummary,
};
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

const SINE_HEALTH_EVERY_STEPS: u32 = 960;
const FOCMAP_SAMPLE_EVERY_STEPS: u32 = 257;

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
    let v_index = (u_index + ((SINE_TABLE_LEN * 2) / 3)) % SINE_TABLE_LEN;
    let w_index = (u_index + (SINE_TABLE_LEN / 3)) % SINE_TABLE_LEN;

    (
        sine_duty(u_index, amplitude),
        sine_duty(v_index, amplitude),
        sine_duty(w_index, amplitude),
    )
}

fn sine_refs_q10_for_index(phase_index: usize) -> (i32, i32, i32) {
    let u_index = phase_index % SINE_TABLE_LEN;
    let v_index = (u_index + ((SINE_TABLE_LEN * 2) / 3)) % SINE_TABLE_LEN;
    let w_index = (u_index + (SINE_TABLE_LEN / 3)) % SINE_TABLE_LEN;

    (
        SINE_Q10[u_index] as i32,
        SINE_Q10[v_index] as i32,
        SINE_Q10[w_index] as i32,
    )
}

fn phase_theta_x10(phase_index: usize) -> u32 {
    ((phase_index as u32) * 3600) / (SINE_TABLE_LEN as u32)
}

fn phase_sector_60(phase_index: usize) -> u32 {
    ((phase_index as u32) * 6) / (SINE_TABLE_LEN as u32)
}

fn abs_i32(value: i32) -> i32 {
    if value < 0 {
        -value
    } else {
        value
    }
}

fn ref_sign(value: i32) -> i32 {
    if value > 0 {
        1
    } else if value < 0 {
        -1
    } else {
        0
    }
}

fn dominant_ref_label(ref_u: i32, ref_v: i32, ref_w: i32) -> &'static str {
    let au = abs_i32(ref_u);
    let av = abs_i32(ref_v);
    let aw = abs_i32(ref_w);

    if au >= av && au >= aw {
        "u"
    } else if av >= au && av >= aw {
        "v"
    } else {
        "w"
    }
}

fn dominant_ref_sign(ref_u: i32, ref_v: i32, ref_w: i32) -> i32 {
    let label = dominant_ref_label(ref_u, ref_v, ref_w);

    if label == "u" {
        ref_sign(ref_u)
    } else if label == "v" {
        ref_sign(ref_v)
    } else {
        ref_sign(ref_w)
    }
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

fn log_focmap_sample(
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
    current_offsets: CurrentSenseOffsets,
) -> crate::current_sense::FocPrepSample {
    let adc = read_adc_snapshot();
    let foc = read_foc_prep_sample(current_offsets);

    let (ref_u, ref_v, ref_w) = sine_refs_q10_for_index(phase_index);
    let theta_x10 = phase_theta_x10(phase_index);
    let sector60 = phase_sector_60(phase_index);
    let dom = dominant_ref_label(ref_u, ref_v, ref_w);
    let dom_s = dominant_ref_sign(ref_u, ref_v, ref_w);

    let op1_a_s = signed_count_sign(foc.op1_a_delta);
    let op1_b_s = signed_count_sign(foc.op1_b_delta);
    let op2_a_s = signed_count_sign(foc.op2_a_delta);
    let op3v_inj_s = signed_count_sign(foc.op3_vopamp3_injected_delta);
    let op3v_reg_s = signed_count_sign(foc.op3_vopamp3_regular_delta);

    let op12_diff = signed_count_diff(foc.op1_a_delta, foc.op2_a_delta);
    let op13v_diff = signed_count_diff(foc.op1_b_delta, foc.op3_vopamp3_injected_delta);

    rprintln!(
        "focmap_ab run={} step={} erev={} phase_idx={} theta_x10={} sector60={} trig=tim1_ch4_center target={} primary_ok={} ok_a={} ok_b={} pair_a=adc1_op1_adc2_op2 pair_b=adc1_op1_adc2_op3v a_wait={} b_wait={} a_cnt0={} a_cnt1={} b_cnt0={} b_cnt1={} a_jeoc1={} a_jeoc2={} a_jeos1={} a_jeos2={} b_jeoc1={} b_jeoc2={} b_jeos1={} b_jeos2={} a_to={} b_to={} a_rail={} b_rail={} ref_u={} ref_v={} ref_w={} dom={} dom_s={} op1_a_jraw={} op1_a_delta={} op1_a_s={} op1_a_abs={} op2_a_jraw={} op2_a_delta={} op2_a_s={} op2_a_abs={} op1_b_jraw={} op1_b_delta={} op1_b_s={} op3v_injected_jraw={} op3v_injected_delta={} op3v_injected_s={} op3v_injected_abs={} op3v_injected_valid={} op3v_regular_raw={} op3v_regular_delta={} op3v_regular_s={} op3v_regular_valid={} op12_diff={} op13v_diff={} op12_sum2={} op13v_sum2={} vbus_raw={} vbus_delta={} temp_raw={} u={} v={} w={} amp={} delay={}",
        run_id,
        step_index,
        electrical_rev,
        phase_index,
        theta_x10,
        sector60,
        foc.target_cnt,
        foc.ok,
        foc.ok_a,
        foc.ok_b,
        foc.a_wait_loops,
        foc.b_wait_loops,
        foc.a_cnt_before_arm,
        foc.a_cnt_after_read,
        foc.b_cnt_before_arm,
        foc.b_cnt_after_read,
        foc.a_adc1_jeoc,
        foc.a_adc2_jeoc,
        foc.a_adc1_jeos,
        foc.a_adc2_jeos,
        foc.b_adc1_jeoc,
        foc.b_adc2_jeoc,
        foc.b_adc1_jeos,
        foc.b_adc2_jeos,
        foc.a_timeout,
        foc.b_timeout,
        foc.a_near_high_rail,
        foc.b_near_high_rail,
        ref_u,
        ref_v,
        ref_w,
        dom,
        dom_s,
        foc.op1_a_raw,
        foc.op1_a_delta,
        op1_a_s,
        signed_count_abs(foc.op1_a_delta),
        foc.op2_a_raw,
        foc.op2_a_delta,
        op2_a_s,
        signed_count_abs(foc.op2_a_delta),
        foc.op1_b_raw,
        foc.op1_b_delta,
        op1_b_s,
        foc.op3_vopamp3_injected_raw,
        foc.op3_vopamp3_injected_delta,
        op3v_inj_s,
        signed_count_abs(foc.op3_vopamp3_injected_delta),
        foc.op3_vopamp3_injected_valid,
        foc.op3_vopamp3_regular_raw,
        foc.op3_vopamp3_regular_delta,
        op3v_reg_s,
        foc.op3_vopamp3_regular_valid,
        op12_diff,
        op13v_diff,
        foc.op12_sum2,
        foc.op13v_sum2,
        adc.vbus_raw,
        adc_delta(adc.vbus_raw, baseline.vbus_raw),
        adc.temp_raw,
        u_duty,
        v_duty,
        w_duty,
        amplitude,
        step_delay
    );

    if foc.ok_a != 1 || foc.ok_b != 1 {
        rprintln!(
            "focdbg_ab run={} step={} a_isr1=0x{:08x} a_isr2=0x{:08x} a_jsqr1=0x{:08x} a_jsqr2=0x{:08x} b_isr1=0x{:08x} b_isr2=0x{:08x} b_jsqr1=0x{:08x} b_jsqr2=0x{:08x}",
            run_id,
            step_index,
            foc.a_adc1_isr,
            foc.a_adc2_isr,
            foc.a_adc1_jsqr,
            foc.a_adc2_jsqr,
            foc.b_adc1_isr,
            foc.b_adc2_isr,
            foc.b_adc1_jsqr,
            foc.b_adc2_jsqr
        );
    }

    foc
}

fn log_foc_sector_summaries(run_id: u32, summaries: &[FocSectorSummary; 6]) {
    let s0 = summaries[0];
    let s1 = summaries[1];
    let s2 = summaries[2];
    let s3 = summaries[3];
    let s4 = summaries[4];
    let s5 = summaries[5];

    rprintln!(
        "focsum_a run={} s0_samples={} s0_ok_samples={} s0_op1_avg={} s0_op2_avg={} s0_op3v_injected_samples={} s0_op3v_injected_avg={} s0_isum2_avg={} s1_samples={} s1_ok_samples={} s1_op1_avg={} s1_op2_avg={} s1_op3v_injected_samples={} s1_op3v_injected_avg={} s1_isum2_avg={} s2_samples={} s2_ok_samples={} s2_op1_avg={} s2_op2_avg={} s2_op3v_injected_samples={} s2_op3v_injected_avg={} s2_isum2_avg={}",
        run_id,
        s0.samples,
        s0.ok_samples,
        s0.op1_avg(),
        s0.op2_avg(),
        s0.op3_vopamp3_injected_samples,
        s0.op3_vopamp3_injected_avg(),
        s0.isum2_avg(),
        s1.samples,
        s1.ok_samples,
        s1.op1_avg(),
        s1.op2_avg(),
        s1.op3_vopamp3_injected_samples,
        s1.op3_vopamp3_injected_avg(),
        s1.isum2_avg(),
        s2.samples,
        s2.ok_samples,
        s2.op1_avg(),
        s2.op2_avg(),
        s2.op3_vopamp3_injected_samples,
        s2.op3_vopamp3_injected_avg(),
        s2.isum2_avg()
    );

    rprintln!(
        "focsum_b run={} s3_samples={} s3_ok_samples={} s3_op1_avg={} s3_op2_avg={} s3_op3v_injected_samples={} s3_op3v_injected_avg={} s3_isum2_avg={} s4_samples={} s4_ok_samples={} s4_op1_avg={} s4_op2_avg={} s4_op3v_injected_samples={} s4_op3v_injected_avg={} s4_isum2_avg={} s5_samples={} s5_ok_samples={} s5_op1_avg={} s5_op2_avg={} s5_op3v_injected_samples={} s5_op3v_injected_avg={} s5_isum2_avg={}",
        run_id,
        s3.samples,
        s3.ok_samples,
        s3.op1_avg(),
        s3.op2_avg(),
        s3.op3_vopamp3_injected_samples,
        s3.op3_vopamp3_injected_avg(),
        s3.isum2_avg(),
        s4.samples,
        s4.ok_samples,
        s4.op1_avg(),
        s4.op2_avg(),
        s4.op3_vopamp3_injected_samples,
        s4.op3_vopamp3_injected_avg(),
        s4.isum2_avg(),
        s5.samples,
        s5.ok_samples,
        s5.op1_avg(),
        s5.op2_avg(),
        s5.op3_vopamp3_injected_samples,
        s5.op3_vopamp3_injected_avg(),
        s5.isum2_avg()
    );
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
    current_offsets: CurrentSenseOffsets,
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
        let foc = read_foc_prep_sample(current_offsets);

        rprintln!(
            "sine run={} step={} erev={} phase_idx={} delay={} amp={} u={} v={} w={} health_ok={} button={} af_ok={} no_phase_overlap={} tim1_sine_ok={} ccer_ok={} pwm_modes_ok={} moe={} dt={} ccr1={} ccr2={} ccr3={} vbus_raw={} vbus_delta={} temp_raw={} temp_delta={} temp_ok={} pot_raw={} foc_primary_ok={} foc_ok_a={} foc_ok_b={} op3v_injected_valid={} op3v_injected_delta={} op3v_regular_valid={} op3v_regular_delta={} timeout={}",
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
            adc.vbus_raw,
            adc_delta(adc.vbus_raw, baseline.vbus_raw),
            adc.temp_raw,
            temp_delta,
            temp_ok,
            adc.pot_raw,
            foc.ok,
            foc.ok_a,
            foc.ok_b,
            foc.op3_vopamp3_injected_valid,
            foc.op3_vopamp3_injected_delta,
            foc.op3_vopamp3_regular_valid,
            foc.op3_vopamp3_regular_delta,
            adc.timeout
        );
    }

    health_ok
}

pub fn run_sine_openloop(run_id: u32, baseline: AdcSnapshot, current_offsets: CurrentSenseOffsets) {
    rprintln!(
        "sine_run_start run={} hold_button_to_run release_to_stop center={} min={} max={} start_amp={} max_amp={} amp_inc_per_erev={} table_len={} align_hold={} start_delay={} min_delay={} dec_per_erev={} max_steps={} log_every={} health_every={} focmap_every={} focmap=injected_ab_adc1_op1_adc2_op2_then_adc1_op1_adc2_op3v focsum=end_of_run_compact_sector_summary",
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
        SINE_HEALTH_EVERY_STEPS,
        FOCMAP_SAMPLE_EVERY_STEPS
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
        current_offsets,
        true,
    );

    log_focmap_sample(
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
        current_offsets,
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
    let mut sector_summaries = [FocSectorSummary::new(); 6];

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
                current_offsets,
                false,
            );

            if health_ok != 1 {
                cycle_ok = 0;
                break;
            }
        }

        if (sine_steps % FOCMAP_SAMPLE_EVERY_STEPS) == 0 {
            let foc = log_focmap_sample(
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
                current_offsets,
            );

            let sector = phase_sector_60(phase_index) as usize;
            update_foc_sector_summary(&mut sector_summaries[sector], foc);
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

    log_foc_sector_summaries(run_id, &sector_summaries);

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
// Version: v0.5.25-injected-ab-op2-op3v
// Created: 2026-06-08
// Generated timestamp: 2026-06-08T18:10:00Z
// ================================================================