// ================================================================
// File: drive.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/drive.rs
// Version: v0.4.10-remove-unused-deadtime-state
// Purpose: Drive-state definitions, TIM1 commutation expectations,
//          drive-pin readback, and phase-overlap safety helpers for
//          the B-G431B-ESC1 STM32G431CB Rust motor-control bring-up.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Learning notes:
//   - DriveState is intentionally a board-level commutation/readback
//     model, not a closed-loop motor-control state machine.
//   - The expected CCER table and expected GPIO pin states are kept
//     together so startup/fault verification stays tied to the exact
//     six-step vector being applied.
//   - read_drive() checks both live output levels and alternate-function
//     mapping, because an incorrect AF setup can make TIM1 configuration
//     look correct while the pins are not actually under timer control.
//   - no_phase_overlap() is a simple hardware-safety sanity check: it
//     verifies high and low outputs of the same phase are not both high.
//   - The old DeadtimeAllOff placeholder was removed because no current
//     code constructed it. Hardware dead-time is handled by TIM1 BDTR;
//     software blanking can be added later as an active, tested state.
// ================================================================

use core::ptr::write_volatile;

use crate::gpio::*;
use crate::regs::*;

// ------------------------------------------------------------
// State definitions
// ------------------------------------------------------------

#[derive(Copy, Clone)]
pub enum DriveState {
    IdleAllOff,
    BootstrapUL,
    BootstrapVL,
    BootstrapWL,
    VectorUhVl,
    VectorUhWl,
    VectorVhWl,
    VectorVhUl,
    VectorWhUl,
    VectorWhVl,
    FaultAllOff,
}

impl DriveState {
    pub fn name(self) -> &'static str {
        match self {
            DriveState::IdleAllOff => "idle_all_off",
            DriveState::BootstrapUL => "bootstrap_ul",
            DriveState::BootstrapVL => "bootstrap_vl",
            DriveState::BootstrapWL => "bootstrap_wl",
            DriveState::VectorUhVl => "vector_uh_vl",
            DriveState::VectorUhWl => "vector_uh_wl",
            DriveState::VectorVhWl => "vector_vh_wl",
            DriveState::VectorVhUl => "vector_vh_ul",
            DriveState::VectorWhUl => "vector_wh_ul",
            DriveState::VectorWhVl => "vector_wh_vl",
            DriveState::FaultAllOff => "fault_all_off",
        }
    }

    pub fn expected_ccer(self) -> u32 {
        match self {
            DriveState::IdleAllOff => TIM1_CCER_ALL_OFF_FORCED_LOW,
            DriveState::BootstrapUL => TIM1_CCER_BOOTSTRAP_U_LOW,
            DriveState::BootstrapVL => TIM1_CCER_BOOTSTRAP_V_LOW,
            DriveState::BootstrapWL => TIM1_CCER_BOOTSTRAP_W_LOW,
            DriveState::VectorUhVl => TIM1_CCER_VECTOR_UH_VL,
            DriveState::VectorUhWl => TIM1_CCER_VECTOR_UH_WL,
            DriveState::VectorVhWl => TIM1_CCER_VECTOR_VH_WL,
            DriveState::VectorVhUl => TIM1_CCER_VECTOR_VH_UL,
            DriveState::VectorWhUl => TIM1_CCER_VECTOR_WH_UL,
            DriveState::VectorWhVl => TIM1_CCER_VECTOR_WH_VL,
            DriveState::FaultAllOff => TIM1_CCER_ALL_OFF_FORCED_LOW,
        }
    }

    pub fn expected_pins(self) -> (u32, u32, u32, u32, u32, u32) {
        match self {
            DriveState::IdleAllOff => (0, 0, 0, 0, 0, 0),
            DriveState::BootstrapUL => (0, 1, 0, 0, 0, 0),
            DriveState::BootstrapVL => (0, 0, 0, 1, 0, 0),
            DriveState::BootstrapWL => (0, 0, 0, 0, 0, 1),
            DriveState::VectorUhVl => (1, 0, 0, 1, 0, 0),
            DriveState::VectorUhWl => (1, 0, 0, 0, 0, 1),
            DriveState::VectorVhWl => (0, 0, 1, 0, 0, 1),
            DriveState::VectorVhUl => (0, 1, 1, 0, 0, 0),
            DriveState::VectorWhUl => (0, 1, 0, 0, 1, 0),
            DriveState::VectorWhVl => (0, 0, 0, 1, 1, 0),
            DriveState::FaultAllOff => (0, 0, 0, 0, 0, 0),
        }
    }

    pub fn ch1_mode(self) -> u32 {
        match self {
            DriveState::VectorUhVl | DriveState::VectorUhWl => TIM1_CCMR_FORCED_ACTIVE,
            _ => TIM1_CCMR_FORCED_INACTIVE,
        }
    }

    pub fn ch2_mode(self) -> u32 {
        match self {
            DriveState::VectorVhWl | DriveState::VectorVhUl => TIM1_CCMR_FORCED_ACTIVE,
            _ => TIM1_CCMR_FORCED_INACTIVE,
        }
    }

    pub fn ch3_mode(self) -> u32 {
        match self {
            DriveState::VectorWhUl | DriveState::VectorWhVl => TIM1_CCMR_FORCED_ACTIVE,
            _ => TIM1_CCMR_FORCED_INACTIVE,
        }
    }

    fn active_led(self) -> bool {
        match self {
            DriveState::BootstrapUL
            | DriveState::BootstrapVL
            | DriveState::BootstrapWL
            | DriveState::VectorUhVl
            | DriveState::VectorUhWl
            | DriveState::VectorVhWl
            | DriveState::VectorVhUl
            | DriveState::VectorWhUl
            | DriveState::VectorWhVl => true,
            _ => false,
        }
    }
}

// ------------------------------------------------------------
// Readback structures
// ------------------------------------------------------------

#[derive(Copy, Clone)]
pub struct DriveReadback {
    pub uh_pin: u32,
    pub ul_pin: u32,
    pub vh_pin: u32,
    pub vl_pin: u32,
    pub wh_pin: u32,
    pub wl_pin: u32,
    pub af_ok: u32,
}

// ------------------------------------------------------------
// Drive GPIO / readback helpers
// ------------------------------------------------------------

pub fn led_for_state(state: DriveState) {
    if state.active_led() {
        led_high();
    } else {
        led_low();
    }
}

pub fn force_drive_output_latches_low() {
    unsafe {
        write_volatile(
            GPIOA_BSRR,
            (1 << (DRIVE_UH_PIN + 16))
                | (1 << (DRIVE_VH_PIN + 16))
                | (1 << (DRIVE_WH_PIN + 16))
                | (1 << (DRIVE_VL_PIN + 16)),
        );

        write_volatile(GPIOB_BSRR, 1 << (DRIVE_WL_PIN + 16));
        write_volatile(GPIOC_BSRR, 1 << (DRIVE_UL_PIN + 16));
    }
}

pub fn read_drive() -> DriveReadback {
    let uh_pin = read_pin(GPIOA_IDR, DRIVE_UH_PIN);
    let vh_pin = read_pin(GPIOA_IDR, DRIVE_VH_PIN);
    let wh_pin = read_pin(GPIOA_IDR, DRIVE_WH_PIN);
    let vl_pin = read_pin(GPIOA_IDR, DRIVE_VL_PIN);
    let wl_pin = read_pin(GPIOB_IDR, DRIVE_WL_PIN);
    let ul_pin = read_pin(GPIOC_IDR, DRIVE_UL_PIN);

    let uh_mode = get_pin_mode(GPIOA_MODER, DRIVE_UH_PIN);
    let vh_mode = get_pin_mode(GPIOA_MODER, DRIVE_VH_PIN);
    let wh_mode = get_pin_mode(GPIOA_MODER, DRIVE_WH_PIN);
    let vl_mode = get_pin_mode(GPIOA_MODER, DRIVE_VL_PIN);
    let wl_mode = get_pin_mode(GPIOB_MODER, DRIVE_WL_PIN);
    let ul_mode = get_pin_mode(GPIOC_MODER, DRIVE_UL_PIN);

    let uh_af = get_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_UH_PIN);
    let vh_af = get_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VH_PIN);
    let wh_af = get_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_WH_PIN);
    let vl_af = get_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VL_PIN);
    let wl_af = get_pin_af(GPIOB_AFRL, GPIOB_AFRH, DRIVE_WL_PIN);
    let ul_af = get_pin_af(GPIOC_AFRL, GPIOC_AFRH, DRIVE_UL_PIN);

    let af_ok = if uh_mode == 0b10
        && vh_mode == 0b10
        && wh_mode == 0b10
        && vl_mode == 0b10
        && wl_mode == 0b10
        && ul_mode == 0b10
        && uh_af == AF_TIM1_PA
        && vh_af == AF_TIM1_PA
        && wh_af == AF_TIM1_PA
        && vl_af == AF_TIM1_PA
        && wl_af == AF_TIM1_N
        && ul_af == AF_TIM1_N
    {
        1
    } else {
        0
    };

    DriveReadback {
        uh_pin,
        ul_pin,
        vh_pin,
        vl_pin,
        wh_pin,
        wl_pin,
        af_ok,
    }
}

pub fn pins_match_state(drive: DriveReadback, state: DriveState) -> u32 {
    let expected = state.expected_pins();

    if drive.uh_pin == expected.0
        && drive.ul_pin == expected.1
        && drive.vh_pin == expected.2
        && drive.vl_pin == expected.3
        && drive.wh_pin == expected.4
        && drive.wl_pin == expected.5
    {
        1
    } else {
        0
    }
}

pub fn no_phase_overlap(drive: DriveReadback) -> u32 {
    if (drive.uh_pin == 1 && drive.ul_pin == 1)
        || (drive.vh_pin == 1 && drive.vl_pin == 1)
        || (drive.wh_pin == 1 && drive.wl_pin == 1)
    {
        0
    } else {
        1
    }
}

pub fn active_pin_count(drive: DriveReadback) -> u32 {
    drive.uh_pin + drive.ul_pin + drive.vh_pin + drive.vl_pin + drive.wh_pin + drive.wl_pin
}

// ================================================================
// Footer
// File: drive.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/drive.rs
// Version: v0.4.10-remove-unused-deadtime-state
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================