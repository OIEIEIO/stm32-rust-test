// ================================================================
// File: tim1.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/tim1.rs
// Version: v0.4.4-split-tim1-same-behavior
// Purpose: TIM1 setup, forced-output state application, PWM vector
//          application, and TIM1 readback verification for the
//          B-G431B-ESC1 STM32G431CB Rust motor-control bring-up.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Learning notes:
//   - TIM1 is the advanced-control timer driving the three half-bridges.
//   - Non-PWM states use forced active/inactive output-compare modes with
//     CCR values held at zero; this keeps startup, bootstrap precharge,
//     idle, and fault states easy to verify.
//   - PWM run vectors only convert the active high-side channel to
//     PWM mode 1. The low-side channel remains complementary solid-on and
//     the floating phase remains disabled by the CCER table.
//   - MOE, OSSI, and OSSR are explicitly asserted whenever a state/vector
//     is applied because advanced timers can otherwise appear configured
//     while outputs remain gated off.
//   - This split is intended to be behavior-preserving; register writes,
//     table usage, timing delays, and LED behavior match the monolithic
//     v0.4.0 open-loop BEMF-observe baseline.
// ================================================================

use core::ptr::{read_volatile, write_volatile};

use cortex_m::asm;

use crate::drive::*;
use crate::regs::*;

// ------------------------------------------------------------
// Local delay helper
// ------------------------------------------------------------

fn delay_cycles(cycles: u32) {
    asm::delay(cycles);
}

// ------------------------------------------------------------
// TIM1 readback structure
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

// ------------------------------------------------------------
// TIM1 helpers
// ------------------------------------------------------------

pub fn set_tim1_modes_for_state(state: DriveState) {
    unsafe {
        let ccmr1 =
            (state.ch1_mode() << 4) |
            (state.ch2_mode() << 12);

        let ccmr2 = state.ch3_mode() << 4;

        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);
    }
}

// Non-PWM states (idle, bootstrap precharge, deadtime, fault).
// High side forced active/inactive, CCR held at 0. Used for the
// startup readback, bootstrap precharge and all dead-man stops.
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
// low-side-active channel stays FORCED_INACTIVE (its complementary low
// FET on); the floating channel stays disabled in CCER. The CCER value
// is the same confirmed table entry used by apply_state. The PWMing
// phase's low FET is never actively driven, so off-time current
// freewheels through its body diode and recharges the bootstrap cap.
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
        let ccmr2 = ch3_mode << 4;
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
// Version: v0.4.4-split-tim1-same-behavior
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================
