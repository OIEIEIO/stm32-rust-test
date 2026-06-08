// ================================================================
// File: opamp.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/opamp.rs
// Version: v0.5.22-op3-vopamp3-diagnostic
// Purpose: Minimal OPAMP1/2/3 setup and readback for B-G431B-ESC1
//          current-sense analog-chain bring-up, with OPAMP3 internal
//          VOPAMP3 routing enabled for diagnostic ADC2 sampling.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.5.4:
//   - Keeps OPAMP1 and OPAMP2 on the already-tested external VOUT path.
//   - Configures OPAMP3 with the same PGA x16 high-speed setup plus the
//     internal OPAMP output routing bit so ADC2 can sample VOPAMP3.
//   - Adds explicit readback fields showing whether OPAMP3 internal routing
//     was actually accepted in OPAMP3_CSR.
//   - No PWM, TIM1, ADC trigger, sine ramp, six-step, FOC, Clarke/Park,
//     current-loop, or SVPWM behavior change.
//
// Change summary vs v0.5.3:
//   - Uses corrected OPAMP2_CSR and OPAMP3_CSR addresses from regs.rs.
//   - Keeps the same conservative PGA x16 high-speed routing.
//
// Bring-up scope:
//   - Enable the SYSCFG/OPAMP clock path.
//   - Configure OPAMP1 and OPAMP2 in PGA mode for external VOUT reads.
//   - Configure OPAMP3 in PGA mode and enable internal VOPAMP3 routing.
//   - Use non-inverting IO0 inputs: PA1, PA7, PB0.
//   - Use gain x16, high-speed mode, factory trim.
//   - Do not add FOC, current loops, Clarke/Park, SVPWM, or sector switching.
//
// Learning notes:
//   - OPAMP CSR configuration is written before setting OPAMPXEN. This avoids
//     switching the analog mux while the amplifier is already driving.
//   - OPAMP1/2/3 CSRs are contiguous registers. If OPAMP2 or OPAMP3 reports
//     an unexpected CSR value, first verify the addresses in regs.rs.
//   - OPAMP3 is special on this board because the B-G431B-ESC1 current-sense
//     model uses three OPAMPs with only two ADCs. This file only enables the
//     internal route for a diagnostic read; it does not reconstruct phase
//     currents yet.
// ================================================================

use core::ptr::{read_volatile, write_volatile};

use crate::regs::*;
use crate::safety::delay_cycles;

#[derive(Copy, Clone)]
pub struct OpampStatus {
    pub csr1: u32,
    pub csr2: u32,
    pub csr3: u32,
    pub en1: u32,
    pub en2: u32,
    pub en3: u32,
    pub cfg1_ok: u32,
    pub cfg2_ok: u32,
    pub cfg3_ok: u32,
    pub op3_vopamp3_internal_ok: u32,
    pub setup_ok: u32,
}

fn cfg_ok(csr: u32, expected_config: u32) -> u32 {
    let expected = expected_config | OPAMP_CSR_OPAMPXEN;
    if (csr & expected) == expected { 1 } else { 0 }
}

pub fn read_opamp_status() -> OpampStatus {
    let csr1;
    let csr2;
    let csr3;

    unsafe {
        csr1 = read_volatile(OPAMP1_CSR);
        csr2 = read_volatile(OPAMP2_CSR);
        csr3 = read_volatile(OPAMP3_CSR);
    }

    let en1 = if (csr1 & OPAMP_CSR_OPAMPXEN) != 0 { 1 } else { 0 };
    let en2 = if (csr2 & OPAMP_CSR_OPAMPXEN) != 0 { 1 } else { 0 };
    let en3 = if (csr3 & OPAMP_CSR_OPAMPXEN) != 0 { 1 } else { 0 };

    let cfg1_ok = cfg_ok(csr1, OPAMP_CURRENT_SENSE_CONFIG);
    let cfg2_ok = cfg_ok(csr2, OPAMP_CURRENT_SENSE_CONFIG);
    let cfg3_ok = cfg_ok(csr3, OPAMP3_VOPAMP3_DIAG_CONFIG);
    let op3_vopamp3_internal_ok = if (csr3 & OPAMP_CSR_OPAMPINTEN) != 0 { 1 } else { 0 };

    let setup_ok = if en1 == 1
        && en2 == 1
        && en3 == 1
        && cfg1_ok == 1
        && cfg2_ok == 1
        && cfg3_ok == 1
        && op3_vopamp3_internal_ok == 1
    {
        1
    } else {
        0
    };

    OpampStatus {
        csr1,
        csr2,
        csr3,
        en1,
        en2,
        en3,
        cfg1_ok,
        cfg2_ok,
        cfg3_ok,
        op3_vopamp3_internal_ok,
        setup_ok,
    }
}

pub fn setup_opamps_for_current_sense() -> OpampStatus {
    unsafe {
        let apb2 = read_volatile(RCC_APB2ENR);
        write_volatile(RCC_APB2ENR, apb2 | RCC_APB2ENR_SYSCFGEN);
    }

    delay_cycles(8_000);

    unsafe {
        write_volatile(OPAMP1_CSR, OPAMP_CURRENT_SENSE_CONFIG);
        write_volatile(OPAMP2_CSR, OPAMP_CURRENT_SENSE_CONFIG);
        write_volatile(OPAMP3_CSR, OPAMP3_VOPAMP3_DIAG_CONFIG);
    }

    delay_cycles(8_000);

    unsafe {
        write_volatile(OPAMP1_CSR, OPAMP_CURRENT_SENSE_CONFIG | OPAMP_CSR_OPAMPXEN);
        write_volatile(OPAMP2_CSR, OPAMP_CURRENT_SENSE_CONFIG | OPAMP_CSR_OPAMPXEN);
        write_volatile(OPAMP3_CSR, OPAMP3_VOPAMP3_DIAG_CONFIG | OPAMP_CSR_OPAMPXEN);
    }

    delay_cycles(160_000);

    read_opamp_status()
}

// ================================================================
// Footer
// File: opamp.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/opamp.rs
// Version: v0.5.22-op3-vopamp3-diagnostic
// Created: 2026-06-08
// Generated timestamp: 2026-06-08T15:45:00Z
// ================================================================