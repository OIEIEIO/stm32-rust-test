// ================================================================
// File: safety.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/safety.rs
// Version: v0.4.7-split-safety-same-behavior
// Purpose: Delay/dead-man safety helpers for the B-G431B-ESC1 Rust
//          open-loop six-step bring-up firmware.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Split note:
//   This file is a behavior-preserving extraction from main.rs.
//   It owns only simple delay and button-held dead-man helpers.
//   The motor drive state table and TIM1 register writes remain in
//   drive.rs / tim1.rs.
//
// Bring-up lesson:
//   Long blocking waits are broken into RELEASE_CHECK_CHUNK pieces so
//   the physical dead-man button can shut the bridge off during align,
//   bootstrap, blanking, and per-step hold periods. This keeps the
//   early open-loop motor tests recoverable even while the code still
//   uses simple busy-wait timing.
// ================================================================

use cortex_m::asm;

use crate::drive::DriveState;
use crate::gpio::button_pressed;
use crate::regs::RELEASE_CHECK_CHUNK;
use crate::tim1::apply_state;

pub fn delay_cycles(cycles: u32) {
    asm::delay(cycles);
}

pub fn delay_while_button_held(total_cycles: u32) -> bool {
    let mut remaining = total_cycles;

    while remaining > 0 {
        if !button_pressed() {
            apply_state(DriveState::IdleAllOff);
            return false;
        }

        let step = if remaining > RELEASE_CHECK_CHUNK {
            RELEASE_CHECK_CHUNK
        } else {
            remaining
        };

        delay_cycles(step);
        remaining -= step;
    }

    true
}

// ================================================================
// Footer
// File: safety.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/safety.rs
// Version: v0.4.7-split-safety-same-behavior
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================
