// ================================================================
// File: gpio.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/gpio.rs
// Version: v0.4.2-split-gpio-same-behavior
// Purpose: Generic GPIO helpers for the B-G431B-ESC1 bring-up.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Learning notes:
//   - Keep these helpers deliberately thin: direct register read/write,
//     no hidden policy and no motor-control decisions.
//   - Pin mode and alternate-function updates are read/modify/write so
//     unrelated pins on the same GPIO port are preserved.
//   - The PC10 user button is active-low on this board.
//   - PC6 is the status LED output used as a visible safety/status cue.
// ================================================================

use core::ptr::{read_volatile, write_volatile};

use crate::regs::*;

pub fn read_pin(idr: *const u32, pin: u32) -> u32 {
    let value = unsafe { read_volatile(idr) };

    if (value & (1 << pin)) != 0 {
        1
    } else {
        0
    }
}

pub fn set_pin_mode(moder: *mut u32, pin: u32, mode: u32) {
    unsafe {
        let mut value = read_volatile(moder);
        value &= !(0b11 << (pin * 2));
        value |= mode << (pin * 2);
        write_volatile(moder, value);
    }
}

pub fn get_pin_mode(moder: *mut u32, pin: u32) -> u32 {
    unsafe { (read_volatile(moder) >> (pin * 2)) & 0b11 }
}

pub fn set_pin_af(afrl: *mut u32, afrh: *mut u32, pin: u32, af: u32) {
    unsafe {
        if pin < 8 {
            let shift = pin * 4;
            let mut value = read_volatile(afrl);
            value &= !(0b1111 << shift);
            value |= af << shift;
            write_volatile(afrl, value);
        } else {
            let shift = (pin - 8) * 4;
            let mut value = read_volatile(afrh);
            value &= !(0b1111 << shift);
            value |= af << shift;
            write_volatile(afrh, value);
        }
    }
}

pub fn get_pin_af(afrl: *mut u32, afrh: *mut u32, pin: u32) -> u32 {
    unsafe {
        if pin < 8 {
            let shift = pin * 4;
            (read_volatile(afrl) >> shift) & 0b1111
        } else {
            let shift = (pin - 8) * 4;
            (read_volatile(afrh) >> shift) & 0b1111
        }
    }
}

pub fn led_high() {
    unsafe {
        write_volatile(GPIOC_BSRR, 1 << STATUS_LED_PIN);
    }
}

pub fn led_low() {
    unsafe {
        write_volatile(GPIOC_BSRR, 1 << (STATUS_LED_PIN + 16));
    }
}

pub fn button_pressed() -> bool {
    let idr = unsafe { read_volatile(GPIOC_IDR) };
    (idr & (1 << USER_BUTTON_PIN)) == 0
}

// ================================================================
// Footer
// File: gpio.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/gpio.rs
// Version: v0.4.2-split-gpio-same-behavior
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================
