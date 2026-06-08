// ================================================================
// File: adc.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/adc.rs
// Version: v0.4.5-split-adc-same-behavior
// Purpose: ADC setup and raw board-monitor/BEMF-channel read helpers
//          for the STM32G431CB B-G431B-ESC1 bring-up firmware.
//
// Learning notes:
//   - ADC1 is used for board monitor channels: pot, temp, VBUS, OP1, OP3.
//   - ADC2 is used for OP2 and the BEMF sense channels.
//   - BEMF channels use a shorter sample-time field so sampling can fit
//     into the PWM-off observation window used by the open-loop firmware.
//   - This module keeps ADC reads polling/blocking, matching the original
//     monolithic main.rs behavior. No DMA, interrupts, or closed-loop
//     commutation are introduced here.
// ================================================================

use core::ptr::{read_volatile, write_volatile};

use cortex_m::asm;

use crate::regs::*;

// ------------------------------------------------------------
// ADC readback structure
// ------------------------------------------------------------

#[derive(Copy, Clone)]
pub struct AdcSnapshot {
    pub pot_raw: u16,
    pub temp_raw: u16,
    pub vbus_raw: u16,
    pub op1_raw: u16,
    pub op2_raw: u16,
    pub op3_raw: u16,
    pub timeout: u32,
}

// ------------------------------------------------------------
// ADC helpers
// ------------------------------------------------------------

pub fn adc_select_channel(adc_base: usize, channel: u32) {
    unsafe {
        let sqr1 = channel << 6;
        write_volatile(adc_sqr1(adc_base), sqr1);
    }
}

pub fn adc_set_sample_time(adc_base: usize, channel: u32) {
    unsafe {
        let sample_bits: u32 = 0b111;

        if channel <= 9 {
            let shift = channel * 3;
            let mut smpr1 = read_volatile(adc_smpr1(adc_base));
            smpr1 &= !(0b111 << shift);
            smpr1 |= sample_bits << shift;
            write_volatile(adc_smpr1(adc_base), smpr1);
        } else {
            let shift = (channel - 10) * 3;
            let mut smpr2 = read_volatile(adc_smpr2(adc_base));
            smpr2 &= !(0b111 << shift);
            smpr2 |= sample_bits << shift;
            write_volatile(adc_smpr2(adc_base), smpr2);
        }
    }
}

// Same as adc_set_sample_time but with a caller-chosen sample-time
// field. Used for the BEMF channels, which need a short sample time
// so the conversion fits inside the PWM-off window.
pub fn adc_set_sample_time_bits(adc_base: usize, channel: u32, bits: u32) {
    unsafe {
        if channel <= 9 {
            let shift = channel * 3;
            let mut smpr1 = read_volatile(adc_smpr1(adc_base));
            smpr1 &= !(0b111 << shift);
            smpr1 |= (bits & 0b111) << shift;
            write_volatile(adc_smpr1(adc_base), smpr1);
        } else {
            let shift = (channel - 10) * 3;
            let mut smpr2 = read_volatile(adc_smpr2(adc_base));
            smpr2 &= !(0b111 << shift);
            smpr2 |= (bits & 0b111) << shift;
            write_volatile(adc_smpr2(adc_base), smpr2);
        }
    }
}

pub fn adc_set_single_ended(adc_base: usize, channel: u32) {
    unsafe {
        let mut difsel = read_volatile(adc_difsel(adc_base));
        difsel &= !(1 << channel);
        write_volatile(adc_difsel(adc_base), difsel);
    }
}

pub fn adc_read_channel_raw(adc_base: usize, channel: u32) -> u16 {
    unsafe {
        adc_select_channel(adc_base, channel);

        write_volatile(adc_isr(adc_base), ADC_ISR_EOC | ADC_ISR_EOS);

        let cr = read_volatile(adc_cr(adc_base));
        write_volatile(adc_cr(adc_base), cr | ADC_CR_ADSTART);

        for _ in 0..1_000_000 {
            let isr = read_volatile(adc_isr(adc_base));

            if (isr & ADC_ISR_EOC) != 0 {
                let raw = read_volatile(adc_dr(adc_base)) & 0x0FFF;
                return raw as u16;
            }
        }
    }

    ADC_TIMEOUT_VALUE
}

pub fn adc_delta(current: u16, baseline: u16) -> i32 {
    (current as i32) - (baseline as i32)
}

pub fn read_adc_snapshot() -> AdcSnapshot {
    let pot_raw = adc_read_channel_raw(ADC1_BASE, POT_ADC_CHANNEL);
    let temp_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
    let vbus_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

    let op1_raw = adc_read_channel_raw(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    let op2_raw = adc_read_channel_raw(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    let op3_raw = adc_read_channel_raw(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    let timeout = if pot_raw == ADC_TIMEOUT_VALUE
        || temp_raw == ADC_TIMEOUT_VALUE
        || vbus_raw == ADC_TIMEOUT_VALUE
        || op1_raw == ADC_TIMEOUT_VALUE
        || op2_raw == ADC_TIMEOUT_VALUE
        || op3_raw == ADC_TIMEOUT_VALUE
    {
        1
    } else {
        0
    };

    AdcSnapshot {
        pot_raw,
        temp_raw,
        vbus_raw,
        op1_raw,
        op2_raw,
        op3_raw,
        timeout,
    }
}

// ------------------------------------------------------------
// ADC setup helpers
// ------------------------------------------------------------

pub fn setup_adc12_common_clock() {
    unsafe {
        let mut ccr = read_volatile(ADC12_CCR);
        ccr &= !(0b11 << 16);
        ccr |= 0b01 << 16;
        write_volatile(ADC12_CCR, ccr);
    }
}

pub fn setup_single_adc(adc_base: usize) -> u32 {
    let mut status: u32 = 0;

    unsafe {
        let mut cr = read_volatile(adc_cr(adc_base));
        cr &= !ADC_CR_DEEPPWD;
        cr |= ADC_CR_ADVREGEN;
        write_volatile(adc_cr(adc_base), cr);

        asm::delay(160_000);

        write_volatile(adc_cfgr(adc_base), 0);

        let cr_before_cal = read_volatile(adc_cr(adc_base));
        write_volatile(adc_cr(adc_base), cr_before_cal | ADC_CR_ADCAL);

        let mut cal_done = false;
        for _ in 0..1_000_000 {
            if (read_volatile(adc_cr(adc_base)) & ADC_CR_ADCAL) == 0 {
                cal_done = true;
                break;
            }
        }

        if !cal_done {
            status |= 1 << 0;
        }

        write_volatile(adc_isr(adc_base), ADC_ISR_ADRDY);

        let cr_before_enable = read_volatile(adc_cr(adc_base));
        write_volatile(adc_cr(adc_base), cr_before_enable | ADC_CR_ADEN);

        let mut ready = false;
        for _ in 0..1_000_000 {
            if (read_volatile(adc_isr(adc_base)) & ADC_ISR_ADRDY) != 0 {
                ready = true;
                break;
            }
        }

        if !ready {
            status |= 1 << 1;
        }
    }

    status
}

pub fn setup_adc1_channels() {
    adc_set_single_ended(ADC1_BASE, VBUS_ADC_CHANNEL);
    adc_set_single_ended(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    adc_set_single_ended(ADC1_BASE, TEMP_ADC_CHANNEL);
    adc_set_single_ended(ADC1_BASE, POT_ADC_CHANNEL);
    adc_set_single_ended(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    adc_set_sample_time(ADC1_BASE, VBUS_ADC_CHANNEL);
    adc_set_sample_time(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    adc_set_sample_time(ADC1_BASE, TEMP_ADC_CHANNEL);
    adc_set_sample_time(ADC1_BASE, POT_ADC_CHANNEL);
    adc_set_sample_time(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    adc_select_channel(ADC1_BASE, POT_ADC_CHANNEL);
}

pub fn setup_adc2_channels() {
    adc_set_single_ended(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    adc_set_sample_time(ADC2_BASE, OP2_OUT_ADC_CHANNEL);

    // BEMF sense channels on ADC2 (fast sample time for in-window reads).
    adc_set_single_ended(ADC2_BASE, BEMF1_ADC_CHANNEL);
    adc_set_single_ended(ADC2_BASE, BEMF2_ADC_CHANNEL);
    adc_set_single_ended(ADC2_BASE, BEMF3_ADC_CHANNEL);
    adc_set_sample_time_bits(ADC2_BASE, BEMF1_ADC_CHANNEL, BEMF_SAMPLE_BITS);
    adc_set_sample_time_bits(ADC2_BASE, BEMF2_ADC_CHANNEL, BEMF_SAMPLE_BITS);
    adc_set_sample_time_bits(ADC2_BASE, BEMF3_ADC_CHANNEL, BEMF_SAMPLE_BITS);

    adc_select_channel(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
}

pub fn setup_adc_for_board_monitor() -> (u32, u32) {
    setup_adc12_common_clock();

    setup_adc1_channels();
    let adc1_status = setup_single_adc(ADC1_BASE);

    setup_adc2_channels();
    let adc2_status = setup_single_adc(ADC2_BASE);

    (adc1_status, adc2_status)
}
// ================================================================
// End of file: adc.rs
// Version: v0.4.5-split-adc-same-behavior
// Generated: 2026-06-07
// Creation date: 2026-06-07
// ================================================================
