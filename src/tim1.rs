// ================================================================
// File: tim1.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/tim1.rs
// Version: v0.5.13-preserve-ch4-ocref
// Purpose: TIM1 setup, forced-output state application, six-step PWM
//          vector application, sine/SPWM complementary PWM helpers,
//          and TIM1 readback verification for the B-G431B-ESC1
//          STM32G431CB Rust motor-control bring-up.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.5.12 injected-debug branch:
//   - Adds configure_tim1_ch4_current_sense_trigger().
//   - Preserves TIM1_CCMR2 CH4 output-compare/reference bits whenever
//     CH3 sine/forced mode is rewritten. This directly addresses the
//     observed log result where ch4 cc4if=1 but ccmr2 stayed 0x00000060.
//   - Keeps CH4 external output disabled; CH4 is internal trigger/debug only.
//   - No intended motor-drive behavior change for CH1/2/3 complementary PWM.
//
// Change summary vs v0.5.0:
//   - Version string unified to the v0.5.2 baseline.
//   - Removed the unused raw readback fields (cnt_a, cnt_b, ccer, bdtr,
//     ccmr1, ccmr2) from Tim1SineReadback to clear the dead_code warning.
//     Those raw registers are still read locally to derive the logged
//     flags (tim1_sine_ok, ccer_ok, pwm_modes_ok, moe, deadtime); only
//     the unlogged raw copies were dropped. The six-step Tim1Readback
//     keeps its equivalent raw fields as the template if raw register
//     dumps are wanted in the sine log later.
//   - No change to PWM generation, dead-time, or any drive behavior.
//
// Change summary vs v0.4.4:
//   - Preserves apply_state(), apply_pwm_vector(), setup_tim1_base(),
//     and read_tim1_for_state() behavior for the existing six-step path.
//   - Adds clamp_duty(), set_tim1_modes_for_sine_pwm(),
//     enable_sine_pwm_outputs(), and apply_sine_pwm_uvw().
//   - Adds read_tim1_for_sine_pwm() for future sine/SPWM logging.
//   - Adds conservative TIM1 BDTR dead-time programming only in the
//     sine/SPWM helper path.
//
// Learning notes:
//   - Six-step mode keeps the existing model: one PWM high side, one
//     solid low side, one floating phase.
//   - Sine/SPWM mode is different: all three phases are PWM driven with
//     CHx/CHxN complementary outputs enabled. There is no floating phase,
//     so BEMF observe data is not meaningful during sine/SPWM.
//   - For complementary PWM, dead-time belongs in TIM1_BDTR.DTG. Software
//     delay windows are not enough once both high and low outputs of each
//     half-bridge are enabled.
// ================================================================

use core::ptr::{read_volatile, write_volatile};

use cortex_m::asm;

use crate::drive::*;
use crate::gpio::led_high;
use crate::regs::*;

// ------------------------------------------------------------
// Local delay helper
// ------------------------------------------------------------

fn delay_cycles(cycles: u32) {
    asm::delay(cycles);
}

// TIM1_CCMR2 contains CH3 and CH4 output-compare mode fields.
// CH3 drives the W phase PWM path. CH4 is kept as an internal
// compare/reference event for current-sense injected-ADC debug.
// Any helper that rewrites CH3 must preserve CH4.
const TIM1_CCMR2_OC3M_MASK_LOCAL: u32 = 0b111 << 4;


// ------------------------------------------------------------
// TIM1 readback structures
// ------------------------------------------------------------

#[derive(Copy, Clone)]
pub struct Tim1Readback {
    pub cnt_a: u32,
    pub cnt_b: u32,
    pub ccer: u32,
    pub bdtr: u32,
    pub ccmr1: u32,
    pub ccmr2: u32,
    pub moe: u32,
    pub forced_modes_ok: u32,
    pub tim1_basic_ok: u32,
}

#[derive(Copy, Clone)]
pub struct Tim1SineReadback {
    pub ccr1: u32,
    pub ccr2: u32,
    pub ccr3: u32,
    pub moe: u32,
    pub pwm_modes_ok: u32,
    pub ccer_ok: u32,
    pub deadtime: u32,
    pub tim1_sine_ok: u32,
}

// ------------------------------------------------------------
// TIM1 mode helpers
// ------------------------------------------------------------

pub fn set_tim1_modes_for_state(state: DriveState) {
    unsafe {
        let ccmr1 =
            (state.ch1_mode() << 4) |
            (state.ch2_mode() << 12);

        let mut ccmr2 = read_volatile(TIM1_CCMR2);
        ccmr2 &= !TIM1_CCMR2_OC3M_MASK_LOCAL;
        ccmr2 |= state.ch3_mode() << 4;

        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);
    }
}

pub fn set_tim1_modes_for_sine_pwm() {
    unsafe {
        let ccmr1 =
            (TIM1_CCMR_PWM_MODE_1 << 4) |
            (TIM1_CCMR_PWM_MODE_1 << 12);

        let mut ccmr2 = read_volatile(TIM1_CCMR2);
        ccmr2 &= !TIM1_CCMR2_OC3M_MASK_LOCAL;
        ccmr2 |= TIM1_CCMR_PWM_MODE_1 << 4;

        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);
    }
}

pub fn configure_tim1_ch4_current_sense_trigger() {
    unsafe {
        // CH4 is used as an internal output-compare/reference event only.
        // No CH4 GPIO pin is configured, and CC4E/CC4NE stay disabled.
        write_volatile(TIM1_CCR4, CURRENT_SENSE_INJECTED_TIM1_CCR4);

        let mut ccmr2 = read_volatile(TIM1_CCMR2);
        ccmr2 &= !(TIM1_CCMR2_CC4S_MASK
            | TIM1_CCMR2_OC4FE
            | TIM1_CCMR2_OC4PE
            | TIM1_CCMR2_OC4M_MASK);
        ccmr2 |= TIM1_CCMR2_OC4_INTERNAL_TRIGGER_CONFIG;
        write_volatile(TIM1_CCMR2, ccmr2);

        let mut ccer = read_volatile(TIM1_CCER);
        ccer &= !TIM1_CCER_CC4_OUTPUT_MASK;
        write_volatile(TIM1_CCER, ccer);
    }
}

fn apply_sine_deadtime_and_moe() {
    unsafe {
        let mut bdtr = read_volatile(TIM1_BDTR);

        bdtr &= !TIM1_BDTR_DTG_MASK;
        bdtr |= TIM1_BDTR_SINE_SAFE_DTG & TIM1_BDTR_DTG_MASK;
        bdtr |= TIM1_BDTR_OSSI;
        bdtr |= TIM1_BDTR_OSSR;
        bdtr |= TIM1_BDTR_MOE;

        write_volatile(TIM1_BDTR, bdtr);
    }
}

pub fn clamp_duty(duty: u32) -> u32 {
    if duty < SINE_PWM_MIN_DUTY {
        SINE_PWM_MIN_DUTY
    } else if duty > SINE_PWM_MAX_DUTY {
        SINE_PWM_MAX_DUTY
    } else {
        duty
    }
}

// ------------------------------------------------------------
// Existing six-step / forced-state helpers
// ------------------------------------------------------------

// Non-PWM states: idle, bootstrap precharge, fault.
// High side forced active/inactive, CCR held at 0. Used for startup
// readback, bootstrap precharge, and all dead-man stops.
pub fn apply_state(state: DriveState) {
    unsafe {
        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        set_tim1_modes_for_state(state);

        write_volatile(TIM1_CCER, state.expected_ccer());

        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr |= TIM1_BDTR_OSSI;
        bdtr |= TIM1_BDTR_OSSR;
        bdtr |= TIM1_BDTR_MOE;
        write_volatile(TIM1_BDTR, bdtr);
    }

    led_for_state(state);
}

// PWM vector apply: the high-side channel the commutation table marks
// FORCED_ACTIVE is switched to PWM_MODE_1 and loaded with `duty`; the
// low-side-active channel stays FORCED_INACTIVE, and the floating phase
// stays disabled in CCER.
pub fn apply_pwm_vector(state: DriveState, duty: u32) {
    let ch1_mode = if state.ch1_mode() == TIM1_CCMR_FORCED_ACTIVE {
        TIM1_CCMR_PWM_MODE_1
    } else {
        TIM1_CCMR_FORCED_INACTIVE
    };
    let ch2_mode = if state.ch2_mode() == TIM1_CCMR_FORCED_ACTIVE {
        TIM1_CCMR_PWM_MODE_1
    } else {
        TIM1_CCMR_FORCED_INACTIVE
    };
    let ch3_mode = if state.ch3_mode() == TIM1_CCMR_FORCED_ACTIVE {
        TIM1_CCMR_PWM_MODE_1
    } else {
        TIM1_CCMR_FORCED_INACTIVE
    };

    unsafe {
        write_volatile(
            TIM1_CCR1,
            if ch1_mode == TIM1_CCMR_PWM_MODE_1 { duty } else { 0 },
        );
        write_volatile(
            TIM1_CCR2,
            if ch2_mode == TIM1_CCMR_PWM_MODE_1 { duty } else { 0 },
        );
        write_volatile(
            TIM1_CCR3,
            if ch3_mode == TIM1_CCMR_PWM_MODE_1 { duty } else { 0 },
        );

        let ccmr1 = (ch1_mode << 4) | (ch2_mode << 12);
        let mut ccmr2 = read_volatile(TIM1_CCMR2);
        ccmr2 &= !TIM1_CCMR2_OC3M_MASK_LOCAL;
        ccmr2 |= ch3_mode << 4;
        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);

        write_volatile(TIM1_CCER, state.expected_ccer());

        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr |= TIM1_BDTR_OSSI;
        bdtr |= TIM1_BDTR_OSSR;
        bdtr |= TIM1_BDTR_MOE;
        write_volatile(TIM1_BDTR, bdtr);
    }

    led_for_state(state);
}

// ------------------------------------------------------------
// New sine/SPWM helpers
// ------------------------------------------------------------

pub fn enable_sine_pwm_outputs() {
    unsafe {
        set_tim1_modes_for_sine_pwm();

        write_volatile(TIM1_CCER, TIM1_CCER_SINE_COMPLEMENTARY_PWM);

        apply_sine_deadtime_and_moe();
    }

    led_high();
}

pub fn apply_sine_pwm_uvw(u_duty: u32, v_duty: u32, w_duty: u32) {
    let u = clamp_duty(u_duty);
    let v = clamp_duty(v_duty);
    let w = clamp_duty(w_duty);

    unsafe {
        write_volatile(TIM1_CCR1, u);
        write_volatile(TIM1_CCR2, v);
        write_volatile(TIM1_CCR3, w);

        set_tim1_modes_for_sine_pwm();

        write_volatile(TIM1_CCER, TIM1_CCER_SINE_COMPLEMENTARY_PWM);

        apply_sine_deadtime_and_moe();
    }

    led_high();
}

// ------------------------------------------------------------
// Readback helpers
// ------------------------------------------------------------

pub fn read_tim1_for_state(state: DriveState) -> Tim1Readback {
    unsafe {
        let cnt_a = read_volatile(TIM1_CNT);
        delay_cycles(16_000);
        let cnt_b = read_volatile(TIM1_CNT);

        let ccr1 = read_volatile(TIM1_CCR1);
        let ccr2 = read_volatile(TIM1_CCR2);
        let ccr3 = read_volatile(TIM1_CCR3);
        let ccer = read_volatile(TIM1_CCER);
        let bdtr = read_volatile(TIM1_BDTR);
        let cr2 = read_volatile(TIM1_CR2);
        let ccmr1 = read_volatile(TIM1_CCMR1);
        let ccmr2 = read_volatile(TIM1_CCMR2);

        let counting = if cnt_a != cnt_b { 1 } else { 0 };
        let moe = if (bdtr & TIM1_BDTR_MOE) != 0 { 1 } else { 0 };
        let ossi = if (bdtr & TIM1_BDTR_OSSI) != 0 { 1 } else { 0 };
        let ossr = if (bdtr & TIM1_BDTR_OSSR) != 0 { 1 } else { 0 };

        let oc1m = (ccmr1 >> 4) & 0b111;
        let oc2m = (ccmr1 >> 12) & 0b111;
        let oc3m = (ccmr2 >> 4) & 0b111;

        let forced_modes_ok = if oc1m == state.ch1_mode()
            && oc2m == state.ch2_mode()
            && oc3m == state.ch3_mode()
        {
            1
        } else {
            0
        };

        let idle_bits = (cr2 >> 8) & 0b11_1111;

        let tim1_basic_ok = if counting == 1
            && ccer == state.expected_ccer()
            && moe == 1
            && ossi == 1
            && ossr == 1
            && forced_modes_ok == 1
            && idle_bits == 0
            && ccr1 == 0
            && ccr2 == 0
            && ccr3 == 0
        {
            1
        } else {
            0
        };

        Tim1Readback {
            cnt_a,
            cnt_b,
            ccer,
            bdtr,
            ccmr1,
            ccmr2,
            moe,
            forced_modes_ok,
            tim1_basic_ok,
        }
    }
}

pub fn read_tim1_for_sine_pwm() -> Tim1SineReadback {
    unsafe {
        let cnt_a = read_volatile(TIM1_CNT);
        delay_cycles(16_000);
        let cnt_b = read_volatile(TIM1_CNT);

        let ccr1 = read_volatile(TIM1_CCR1);
        let ccr2 = read_volatile(TIM1_CCR2);
        let ccr3 = read_volatile(TIM1_CCR3);
        let ccer = read_volatile(TIM1_CCER);
        let bdtr = read_volatile(TIM1_BDTR);
        let ccmr1 = read_volatile(TIM1_CCMR1);
        let ccmr2 = read_volatile(TIM1_CCMR2);

        let counting = if cnt_a != cnt_b { 1 } else { 0 };
        let moe = if (bdtr & TIM1_BDTR_MOE) != 0 { 1 } else { 0 };

        let oc1m = (ccmr1 >> 4) & 0b111;
        let oc2m = (ccmr1 >> 12) & 0b111;
        let oc3m = (ccmr2 >> 4) & 0b111;

        let pwm_modes_ok = if oc1m == TIM1_CCMR_PWM_MODE_1
            && oc2m == TIM1_CCMR_PWM_MODE_1
            && oc3m == TIM1_CCMR_PWM_MODE_1
        {
            1
        } else {
            0
        };

        let ccer_ok = if ccer == TIM1_CCER_SINE_COMPLEMENTARY_PWM {
            1
        } else {
            0
        };

        let deadtime = bdtr & TIM1_BDTR_DTG_MASK;

        let tim1_sine_ok = if counting == 1
            && moe == 1
            && pwm_modes_ok == 1
            && ccer_ok == 1
            && deadtime == TIM1_BDTR_SINE_SAFE_DTG
            && ccr1 >= SINE_PWM_MIN_DUTY
            && ccr1 <= SINE_PWM_MAX_DUTY
            && ccr2 >= SINE_PWM_MIN_DUTY
            && ccr2 <= SINE_PWM_MAX_DUTY
            && ccr3 >= SINE_PWM_MIN_DUTY
            && ccr3 <= SINE_PWM_MAX_DUTY
        {
            1
        } else {
            0
        };

        Tim1SineReadback {
            ccr1,
            ccr2,
            ccr3,
            moe,
            pwm_modes_ok,
            ccer_ok,
            deadtime,
            tim1_sine_ok,
        }
    }
}

// ------------------------------------------------------------
// TIM1 base setup
// ------------------------------------------------------------

pub fn setup_tim1_base() {
    unsafe {
        let apb2enr = read_volatile(RCC_APB2ENR);
        write_volatile(RCC_APB2ENR, apb2enr | RCC_APB2ENR_TIM1EN);

        delay_cycles(8_000);

        write_volatile(TIM1_CR1, 0);
        write_volatile(TIM1_CCER, 0);

        let mut cr2 = read_volatile(TIM1_CR2);
        cr2 &= !(0b11_1111 << 8);
        write_volatile(TIM1_CR2, cr2);

        write_volatile(TIM1_PSC, TIM1_TEST_PSC);
        write_volatile(TIM1_ARR, TIM1_TEST_ARR);
        write_volatile(TIM1_RCR, 0);

        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);
        write_volatile(TIM1_CCR4, CURRENT_SENSE_INJECTED_TIM1_CCR4);

        set_tim1_modes_for_state(DriveState::IdleAllOff);

        write_volatile(TIM1_EGR, TIM1_EGR_UG);
        write_volatile(TIM1_CR1, TIM1_CR1_ARPE | TIM1_CR1_CEN);

        apply_state(DriveState::IdleAllOff);
    }
}

// ================================================================
// Footer
// File: tim1.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/tim1.rs
// Version: v0.5.13-preserve-ch4-ocref
// Created: 2026-06-07
// Generated timestamp: 2026-06-08T08:12:00Z
// ================================================================