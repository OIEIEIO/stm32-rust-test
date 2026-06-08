// ================================================================
// File: current_sense.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/current_sense.rs
// Version: v0.5.24-warning-cleanup
// Purpose: Zero-offset capture, raw OPAMP current-sense observation,
//          compact TIM1-triggered injected ADC FOC-prep sampling,
//          OP3/VOPAMP3 regular-ADC diagnostics, and sector-summary
//          helpers for the B-G431B-ESC1 bring-up firmware.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.5.23:
//   - Removes unused average helper methods and avg_u32() that were left
//     from earlier sector-summary diagnostic experiments.
//   - Keeps existing FocSectorSummary fields and update behavior intact.
//   - No sampling, logging, ADC, TIM1, OPAMP, or drive behavior change.
//
// Change summary vs v0.5.22:
//   - Adds an OP3/VOPAMP3 diagnostic read path using ADC2 channel 18.
//   - Keeps the default TIM1_CH4 injected pair unchanged: ADC1=OP1
//     external VOUT and ADC2=OP2 external VOUT.
//   - Captures a separate VOPAMP3 zero offset and reports separate
//     diagnostic timeout/valid fields so OP1/OP2 FOC-prep logging is not
//     broken if the new internal route is wrong.
//   - Adds optional OP3/VOPAMP3 fields to FocPrepSample and
//     FocSectorSummary for logging.
//   - No motor-drive behavior change.
//
// Learning notes:
//   - FOC needs current samples tied to a known PWM timing point, not
//     arbitrary software ADC reads.
//   - The current injected pair is OP1/OP2 only.
//   - OP3/VOPAMP3 is currently a regular ADC diagnostic read. It is not
//     used yet for phase-current reconstruction and does not rename OP1/OP2
//     to Ia/Ib.
// ================================================================

use core::ptr::{read_volatile, write_volatile};

use crate::adc::*;
use crate::regs::*;
use crate::safety::delay_cycles;
use crate::tim1::configure_tim1_ch4_current_sense_trigger;

#[derive(Copy, Clone)]
pub struct CurrentSenseOffsets {
    pub op1_zero: u16,
    pub op2_zero: u16,
    pub op3_zero: u16,
    pub op3_vopamp3_zero: u16,
    pub samples_requested: u32,
    pub samples_used: u32,
    pub timeout: u32,
    pub op3_vopamp3_samples_used: u32,
    pub op3_vopamp3_timeout: u32,
}

#[derive(Copy, Clone)]
pub struct CurrentSenseReading {
    pub op1_raw: u16,
    pub op2_raw: u16,
    pub op3_raw: u16,
    pub op3_vopamp3_raw: u16,
    pub op1_zero: u16,
    pub op2_zero: u16,
    pub op3_zero: u16,
    pub op3_vopamp3_zero: u16,
    pub op1_delta: i32,
    pub op2_delta: i32,
    pub op3_delta: i32,
    pub op3_vopamp3_delta: i32,
    pub near_high_rail: u32,
    pub timeout: u32,
    pub ok: u32,
    pub op3_vopamp3_timeout: u32,
    pub op3_vopamp3_ok: u32,
}

#[derive(Copy, Clone)]
pub struct CurrentSenseInjectedConfig {
    pub target_cnt: u32,
    pub tim1_ccr4: u32,
    pub tim1_ccmr2: u32,
    pub tim1_ccer: u32,
    pub tim1_ch4_oc_ok: u32,
    pub adc1_jsqr: u32,
    pub adc2_jsqr: u32,
    pub adc2_channel: u32,
    pub adc1_cfgr: u32,
    pub adc2_cfgr: u32,
    pub setup_ok: u32,
}

#[derive(Copy, Clone)]
pub struct FocPrepSample {
    pub target_cnt: u32,
    pub cnt_before_arm: u32,
    pub cnt_after_read: u32,
    pub wait_loops: u32,

    pub op1_raw: u16,
    pub op2_raw: u16,
    pub op3_vopamp3_raw: u16,
    pub op1_delta: i32,
    pub op2_delta: i32,
    pub op3_vopamp3_delta: i32,
    pub op_sum2: i32,
    pub op3_vopamp3_valid: u32,

    pub adc1_jeoc: u32,
    pub adc1_jeos: u32,
    pub adc2_jeoc: u32,
    pub adc2_jeos: u32,
    pub adc1_isr: u32,
    pub adc2_isr: u32,
    pub adc1_jsqr: u32,
    pub adc2_jsqr: u32,

    pub timeout: u32,
    pub near_high_rail: u32,
    pub ok: u32,
}

#[derive(Copy, Clone)]
pub struct FocSectorSummary {
    pub samples: u32,
    pub ok_samples: u32,
    pub timeout_samples: u32,
    pub rail_samples: u32,

    pub op1_sum: i32,
    pub op1_min: i32,
    pub op1_max: i32,
    pub op1_abs_sum: u32,

    pub op2_sum: i32,
    pub op2_min: i32,
    pub op2_max: i32,
    pub op2_abs_sum: u32,

    pub op3_vopamp3_samples: u32,
    pub op3_vopamp3_sum: i32,
    pub op3_vopamp3_min: i32,
    pub op3_vopamp3_max: i32,
    pub op3_vopamp3_abs_sum: u32,

    pub isum2_sum: i32,
    pub isum2_min: i32,
    pub isum2_max: i32,
    pub isum2_abs_sum: u32,
}

impl FocSectorSummary {
    pub const fn new() -> Self {
        Self {
            samples: 0,
            ok_samples: 0,
            timeout_samples: 0,
            rail_samples: 0,

            op1_sum: 0,
            op1_min: 0,
            op1_max: 0,
            op1_abs_sum: 0,

            op2_sum: 0,
            op2_min: 0,
            op2_max: 0,
            op2_abs_sum: 0,

            op3_vopamp3_samples: 0,
            op3_vopamp3_sum: 0,
            op3_vopamp3_min: 0,
            op3_vopamp3_max: 0,
            op3_vopamp3_abs_sum: 0,

            isum2_sum: 0,
            isum2_min: 0,
            isum2_max: 0,
            isum2_abs_sum: 0,
        }
    }

    pub fn op1_avg(&self) -> i32 {
        avg_i32(self.op1_sum, self.ok_samples)
    }

    pub fn op2_avg(&self) -> i32 {
        avg_i32(self.op2_sum, self.ok_samples)
    }

    pub fn isum2_avg(&self) -> i32 {
        avg_i32(self.isum2_sum, self.ok_samples)
    }

    pub fn op3_vopamp3_avg(&self) -> i32 {
        avg_i32(self.op3_vopamp3_sum, self.op3_vopamp3_samples)
    }
}

fn avg_i32(sum: i32, count: u32) -> i32 {
    if count == 0 {
        0
    } else {
        sum / (count as i32)
    }
}

pub fn update_foc_sector_summary(summary: &mut FocSectorSummary, sample: FocPrepSample) {
    summary.samples = summary.samples.wrapping_add(1);

    if sample.timeout != 0 {
        summary.timeout_samples = summary.timeout_samples.wrapping_add(1);
    }

    if sample.near_high_rail != 0 {
        summary.rail_samples = summary.rail_samples.wrapping_add(1);
    }

    if sample.ok != 1 {
        return;
    }

    let first_ok = summary.ok_samples == 0;
    summary.ok_samples = summary.ok_samples.wrapping_add(1);

    let op1 = sample.op1_delta;
    let op2 = sample.op2_delta;
    let isum2 = sample.op_sum2;

    summary.op1_sum = summary.op1_sum.wrapping_add(op1);
    summary.op2_sum = summary.op2_sum.wrapping_add(op2);
    summary.isum2_sum = summary.isum2_sum.wrapping_add(isum2);

    summary.op1_abs_sum = summary.op1_abs_sum.wrapping_add(signed_count_abs(op1));
    summary.op2_abs_sum = summary.op2_abs_sum.wrapping_add(signed_count_abs(op2));
    summary.isum2_abs_sum = summary.isum2_abs_sum.wrapping_add(signed_count_abs(isum2));

    if first_ok {
        summary.op1_min = op1;
        summary.op1_max = op1;
        summary.op2_min = op2;
        summary.op2_max = op2;
        summary.isum2_min = isum2;
        summary.isum2_max = isum2;
    } else {
        if op1 < summary.op1_min {
            summary.op1_min = op1;
        }
        if op1 > summary.op1_max {
            summary.op1_max = op1;
        }
        if op2 < summary.op2_min {
            summary.op2_min = op2;
        }
        if op2 > summary.op2_max {
            summary.op2_max = op2;
        }
        if isum2 < summary.isum2_min {
            summary.isum2_min = isum2;
        }
        if isum2 > summary.isum2_max {
            summary.isum2_max = isum2;
        }
    }

    if sample.op3_vopamp3_valid == 1 {
        let op3 = sample.op3_vopamp3_delta;
        let first_op3 = summary.op3_vopamp3_samples == 0;

        summary.op3_vopamp3_samples = summary.op3_vopamp3_samples.wrapping_add(1);
        summary.op3_vopamp3_sum = summary.op3_vopamp3_sum.wrapping_add(op3);
        summary.op3_vopamp3_abs_sum = summary
            .op3_vopamp3_abs_sum
            .wrapping_add(signed_count_abs(op3));

        if first_op3 {
            summary.op3_vopamp3_min = op3;
            summary.op3_vopamp3_max = op3;
        } else {
            if op3 < summary.op3_vopamp3_min {
                summary.op3_vopamp3_min = op3;
            }
            if op3 > summary.op3_vopamp3_max {
                summary.op3_vopamp3_max = op3;
            }
        }
    }
}

// ------------------------------------------------------------
// Phase-mapping observation helpers
// ------------------------------------------------------------

pub const CURRENT_SENSE_PHASE_MAP_SIGN_DEADBAND_COUNTS: i32 = 4;

pub fn signed_count_abs(value: i32) -> u32 {
    if value < 0 {
        (-value) as u32
    } else {
        value as u32
    }
}

pub fn signed_count_sign(value: i32) -> i32 {
    if value > CURRENT_SENSE_PHASE_MAP_SIGN_DEADBAND_COUNTS {
        1
    } else if value < -CURRENT_SENSE_PHASE_MAP_SIGN_DEADBAND_COUNTS {
        -1
    } else {
        0
    }
}

pub fn signed_count_diff(a: i32, b: i32) -> i32 {
    a - b
}

pub fn configure_current_sense_adc_sample_time_for_sync() {
    adc_set_sample_time_bits(ADC1_BASE, OP1_OUT_ADC_CHANNEL, CURRENT_SENSE_SYNC_SAMPLE_BITS);
    adc_set_sample_time_bits(ADC2_BASE, OP2_OUT_ADC_CHANNEL, CURRENT_SENSE_SYNC_SAMPLE_BITS);
    adc_set_sample_time_bits(ADC1_BASE, OP3_OUT_ADC_CHANNEL, CURRENT_SENSE_SYNC_SAMPLE_BITS);
    adc_set_sample_time_bits(ADC2_BASE, OP3_INTERNAL_VOPAMP3_ADC2_CHANNEL, CURRENT_SENSE_SYNC_SAMPLE_BITS);
}

fn injected_jsqr_one_rank_tim1_ch4(channel: u32) -> u32 {
    ADC_JSQR_JEXTSEL_TIM1_CH4
        | ADC_JSQR_JEXTEN_RISING
        | ((channel & 0x1F) << ADC_JSQR_JSQ1_SHIFT)
}

pub fn refresh_tim1_ch4_internal_trigger_config() {
    configure_tim1_ch4_current_sense_trigger();
}

pub fn configure_current_sense_injected_tim1_ch4() -> CurrentSenseInjectedConfig {
    configure_current_sense_adc_sample_time_for_sync();

    let adc1_jsqr = injected_jsqr_one_rank_tim1_ch4(OP1_OUT_ADC_CHANNEL);
    let adc2_jsqr = injected_jsqr_one_rank_tim1_ch4(OP2_OUT_ADC_CHANNEL);

    unsafe {
        refresh_tim1_ch4_internal_trigger_config();
        write_volatile(TIM1_EGR, TIM1_EGR_UG);

        write_volatile(adc_isr(ADC1_BASE), ADC_ISR_JEOC | ADC_ISR_JEOS | ADC_ISR_JQOVF);
        write_volatile(adc_isr(ADC2_BASE), ADC_ISR_JEOC | ADC_ISR_JEOS | ADC_ISR_JQOVF);

        write_volatile(adc_jsqr(ADC1_BASE), adc1_jsqr);
        write_volatile(adc_jsqr(ADC2_BASE), adc2_jsqr);

        let tim1_ccr4 = read_volatile(TIM1_CCR4);
        let tim1_ccmr2 = read_volatile(TIM1_CCMR2);
        let tim1_ccer = read_volatile(TIM1_CCER);
        let adc1_jsqr_rb = read_volatile(adc_jsqr(ADC1_BASE));
        let adc2_jsqr_rb = read_volatile(adc_jsqr(ADC2_BASE));
        let adc1_cfgr_rb = read_volatile(adc_cfgr(ADC1_BASE));
        let adc2_cfgr_rb = read_volatile(adc_cfgr(ADC2_BASE));

        let tim1_ch4_oc_ok = if (tim1_ccmr2 & (TIM1_CCMR2_CC4S_MASK | TIM1_CCMR2_OC4M_MASK))
                == TIM1_CCMR2_OC4_INTERNAL_TRIGGER_CONFIG
            && (tim1_ccer & TIM1_CCER_CC4_OUTPUT_MASK) == 0
        {
            1
        } else {
            0
        };

        let setup_ok = if tim1_ccr4 == CURRENT_SENSE_INJECTED_TIM1_CCR4
            && tim1_ch4_oc_ok == 1
            && adc1_jsqr_rb == adc1_jsqr
            && adc2_jsqr_rb == adc2_jsqr
            && (adc1_cfgr_rb & ADC_CFGR_JQDIS) != 0
            && (adc2_cfgr_rb & ADC_CFGR_JQDIS) != 0
        {
            1
        } else {
            0
        };

        CurrentSenseInjectedConfig {
            target_cnt: CURRENT_SENSE_INJECTED_TIM1_CCR4,
            tim1_ccr4,
            tim1_ccmr2,
            tim1_ccer,
            tim1_ch4_oc_ok,
            adc1_jsqr: adc1_jsqr_rb,
            adc2_jsqr: adc2_jsqr_rb,
            adc2_channel: OP2_OUT_ADC_CHANNEL,
            adc1_cfgr: adc1_cfgr_rb,
            adc2_cfgr: adc2_cfgr_rb,
            setup_ok,
        }
    }
}

fn tim1_counter_raw() -> u32 {
    unsafe { read_volatile(TIM1_CNT) & 0xFFFF }
}

fn read_current_sense_raw_triplet() -> (u16, u16, u16, u32) {
    let op1_raw = adc_read_channel_raw(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    let op2_raw = adc_read_channel_raw(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    let op3_raw = adc_read_channel_raw(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    let timeout = if op1_raw == ADC_TIMEOUT_VALUE
        || op2_raw == ADC_TIMEOUT_VALUE
        || op3_raw == ADC_TIMEOUT_VALUE
    {
        1
    } else {
        0
    };

    (op1_raw, op2_raw, op3_raw, timeout)
}

fn read_op3_vopamp3_internal_raw() -> (u16, u32) {
    let raw = adc_read_channel_raw(ADC2_BASE, OP3_INTERNAL_VOPAMP3_ADC2_CHANNEL);
    let timeout = if raw == ADC_TIMEOUT_VALUE { 1 } else { 0 };

    (raw, timeout)
}

pub fn capture_current_sense_offsets(samples: u32) -> CurrentSenseOffsets {
    let requested = if samples == 0 { 1 } else { samples };

    delay_cycles(CURRENT_SENSE_PRE_OFFSET_SETTLE_DELAY);

    for _ in 0..CURRENT_SENSE_DUMMY_READS_BEFORE_OFFSET {
        let _ = read_adc_snapshot();
        delay_cycles(CURRENT_SENSE_OFFSET_SAMPLE_DELAY);
    }

    let mut op1_sum: u32 = 0;
    let mut op2_sum: u32 = 0;
    let mut op3_sum: u32 = 0;
    let mut op3_vopamp3_sum: u32 = 0;
    let mut used: u32 = 0;
    let mut op3_vopamp3_used: u32 = 0;

    for _ in 0..requested {
        let adc = read_adc_snapshot();
        let (op3_vopamp3_raw, op3_vopamp3_timeout) = read_op3_vopamp3_internal_raw();

        if op3_vopamp3_timeout == 0 {
            op3_vopamp3_sum = op3_vopamp3_sum.wrapping_add(op3_vopamp3_raw as u32);
            op3_vopamp3_used = op3_vopamp3_used.wrapping_add(1);
        }

        if adc.timeout == 0
            && adc.op1_raw != ADC_TIMEOUT_VALUE
            && adc.op2_raw != ADC_TIMEOUT_VALUE
            && adc.op3_raw != ADC_TIMEOUT_VALUE
        {
            op1_sum = op1_sum.wrapping_add(adc.op1_raw as u32);
            op2_sum = op2_sum.wrapping_add(adc.op2_raw as u32);
            op3_sum = op3_sum.wrapping_add(adc.op3_raw as u32);
            used = used.wrapping_add(1);
        }

        delay_cycles(CURRENT_SENSE_OFFSET_SAMPLE_DELAY);
    }

    if used == 0 {
        let adc = read_adc_snapshot();
        return CurrentSenseOffsets {
            op1_zero: adc.op1_raw,
            op2_zero: adc.op2_raw,
            op3_zero: adc.op3_raw,
            op3_vopamp3_zero: ADC_TIMEOUT_VALUE,
            samples_requested: requested,
            samples_used: 0,
            timeout: 1,
            op3_vopamp3_samples_used: op3_vopamp3_used,
            op3_vopamp3_timeout: 1,
        };
    }

    CurrentSenseOffsets {
        op1_zero: (op1_sum / used) as u16,
        op2_zero: (op2_sum / used) as u16,
        op3_zero: (op3_sum / used) as u16,
        op3_vopamp3_zero: if op3_vopamp3_used == 0 {
            ADC_TIMEOUT_VALUE
        } else {
            (op3_vopamp3_sum / op3_vopamp3_used) as u16
        },
        samples_requested: requested,
        samples_used: used,
        timeout: 0,
        op3_vopamp3_samples_used: op3_vopamp3_used,
        op3_vopamp3_timeout: if op3_vopamp3_used == 0 { 1 } else { 0 },
    }
}

pub fn adc_baseline_with_current_offsets(seed: AdcSnapshot, offsets: CurrentSenseOffsets) -> AdcSnapshot {
    AdcSnapshot {
        pot_raw: seed.pot_raw,
        temp_raw: seed.temp_raw,
        vbus_raw: seed.vbus_raw,
        op1_raw: offsets.op1_zero,
        op2_raw: offsets.op2_zero,
        op3_raw: offsets.op3_zero,
        timeout: seed.timeout | offsets.timeout,
    }
}

pub fn read_current_sense(offsets: CurrentSenseOffsets) -> CurrentSenseReading {
    let (op1_raw, op2_raw, op3_raw, adc_timeout) = read_current_sense_raw_triplet();
    let (op3_vopamp3_raw, op3_vopamp3_adc_timeout) = read_op3_vopamp3_internal_raw();

    let op1_delta = adc_delta(op1_raw, offsets.op1_zero);
    let op2_delta = adc_delta(op2_raw, offsets.op2_zero);
    let op3_delta = adc_delta(op3_raw, offsets.op3_zero);
    let op3_vopamp3_delta = adc_delta(op3_vopamp3_raw, offsets.op3_vopamp3_zero);

    let near_high_rail = if op1_raw >= CURRENT_SENSE_NEAR_HIGH_RAIL_RAW
        || op2_raw >= CURRENT_SENSE_NEAR_HIGH_RAIL_RAW
        || op3_raw >= CURRENT_SENSE_NEAR_HIGH_RAIL_RAW
    {
        1
    } else {
        0
    };

    let timeout = adc_timeout | offsets.timeout;
    let op3_vopamp3_timeout = op3_vopamp3_adc_timeout | offsets.op3_vopamp3_timeout;
    let ok = if timeout == 0 && near_high_rail == 0 { 1 } else { 0 };
    let op3_vopamp3_ok = if op3_vopamp3_timeout == 0
        && op3_vopamp3_raw < CURRENT_SENSE_NEAR_HIGH_RAIL_RAW
    {
        1
    } else {
        0
    };

    CurrentSenseReading {
        op1_raw,
        op2_raw,
        op3_raw,
        op3_vopamp3_raw,
        op1_zero: offsets.op1_zero,
        op2_zero: offsets.op2_zero,
        op3_zero: offsets.op3_zero,
        op3_vopamp3_zero: offsets.op3_vopamp3_zero,
        op1_delta,
        op2_delta,
        op3_delta,
        op3_vopamp3_delta,
        near_high_rail,
        timeout,
        ok,
        op3_vopamp3_timeout,
        op3_vopamp3_ok,
    }
}

pub fn read_foc_prep_sample(offsets: CurrentSenseOffsets) -> FocPrepSample {
    let cnt_before_arm = tim1_counter_raw();

    unsafe {
        refresh_tim1_ch4_internal_trigger_config();

        write_volatile(adc_isr(ADC1_BASE), ADC_ISR_JEOC | ADC_ISR_JEOS | ADC_ISR_JQOVF);
        write_volatile(adc_isr(ADC2_BASE), ADC_ISR_JEOC | ADC_ISR_JEOS | ADC_ISR_JQOVF);

        let cr1 = read_volatile(adc_cr(ADC1_BASE));
        let cr2 = read_volatile(adc_cr(ADC2_BASE));
        write_volatile(adc_cr(ADC1_BASE), cr1 | ADC_CR_JADSTART);
        write_volatile(adc_cr(ADC2_BASE), cr2 | ADC_CR_JADSTART);

        let mut adc1_jeoc: u32 = 0;
        let mut adc1_jeos: u32 = 0;
        let mut adc2_jeoc: u32 = 0;
        let mut adc2_jeos: u32 = 0;
        let mut adc1_isr: u32 = 0;
        let mut adc2_isr: u32 = 0;
        let mut wait_loops: u32 = 0;

        for loops in 0..CURRENT_SENSE_INJECTED_WAIT_MAX_LOOPS {
            wait_loops = loops;
            adc1_isr = read_volatile(adc_isr(ADC1_BASE));
            adc2_isr = read_volatile(adc_isr(ADC2_BASE));

            adc1_jeoc = if (adc1_isr & ADC_ISR_JEOC) != 0 { 1 } else { 0 };
            adc1_jeos = if (adc1_isr & ADC_ISR_JEOS) != 0 { 1 } else { 0 };
            adc2_jeoc = if (adc2_isr & ADC_ISR_JEOC) != 0 { 1 } else { 0 };
            adc2_jeos = if (adc2_isr & ADC_ISR_JEOS) != 0 { 1 } else { 0 };

            if adc1_jeos == 1 && adc2_jeos == 1 {
                break;
            }
        }

        let timeout = if adc1_jeos == 1 && adc2_jeos == 1 { 0 } else { 1 };

        if timeout != 0 {
            let cr1_stop = read_volatile(adc_cr(ADC1_BASE));
            let cr2_stop = read_volatile(adc_cr(ADC2_BASE));
            write_volatile(adc_cr(ADC1_BASE), cr1_stop | ADC_CR_JADSTP);
            write_volatile(adc_cr(ADC2_BASE), cr2_stop | ADC_CR_JADSTP);
        }

        let op1_raw = (read_volatile(adc_jdr1(ADC1_BASE)) & 0x0FFF) as u16;
        let op2_raw = (read_volatile(adc_jdr1(ADC2_BASE)) & 0x0FFF) as u16;
        let cnt_after_read = tim1_counter_raw();
        let (op3_vopamp3_raw, op3_vopamp3_timeout) = read_op3_vopamp3_internal_raw();

        let op1_delta = adc_delta(op1_raw, offsets.op1_zero);
        let op2_delta = adc_delta(op2_raw, offsets.op2_zero);
        let op3_vopamp3_delta = adc_delta(op3_vopamp3_raw, offsets.op3_vopamp3_zero);

        let near_high_rail = if op1_raw >= CURRENT_SENSE_NEAR_HIGH_RAIL_RAW
            || op2_raw >= CURRENT_SENSE_NEAR_HIGH_RAIL_RAW
        {
            1
        } else {
            0
        };

        let op3_vopamp3_valid = if offsets.op3_vopamp3_timeout == 0
            && op3_vopamp3_timeout == 0
            && op3_vopamp3_raw < CURRENT_SENSE_NEAR_HIGH_RAIL_RAW
        {
            1
        } else {
            0
        };

        let ok = if offsets.timeout == 0
            && timeout == 0
            && adc1_jeos == 1
            && adc2_jeos == 1
            && near_high_rail == 0
        {
            1
        } else {
            0
        };

        FocPrepSample {
            target_cnt: CURRENT_SENSE_INJECTED_TIM1_CCR4,
            cnt_before_arm,
            cnt_after_read,
            wait_loops,

            op1_raw,
            op2_raw,
            op3_vopamp3_raw,
            op1_delta,
            op2_delta,
            op3_vopamp3_delta,
            op_sum2: op1_delta + op2_delta,
            op3_vopamp3_valid,

            adc1_jeoc,
            adc1_jeos,
            adc2_jeoc,
            adc2_jeos,
            adc1_isr,
            adc2_isr,
            adc1_jsqr: read_volatile(adc_jsqr(ADC1_BASE)),
            adc2_jsqr: read_volatile(adc_jsqr(ADC2_BASE)),

            timeout: timeout | offsets.timeout,
            near_high_rail,
            ok,
        }
    }
}

// ================================================================
// Footer
// File: current_sense.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/current_sense.rs
// Version: v0.5.24-warning-cleanup
// Created: 2026-06-08
// Generated timestamp: 2026-06-08T17:10:00Z
// ================================================================