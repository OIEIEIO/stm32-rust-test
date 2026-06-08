// ================================================================
// File: regs.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/regs.rs
// Version: v0.4.1-split-regs-same-behavior
// Purpose: Register addresses, bit masks, board pin IDs, ADC channel IDs,
//          and small raw-register address helpers for the B-G431B-ESC1
//          STM32G431CB bring-up firmware.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Split note:
//   This file is a mechanical extraction from main.rs v0.4.0.
//   It must not change register values, pin mappings, timing constants,
//   PWM constants, BEMF channel mappings, or ADC helper offsets.
//
// Learning notes:
//   - Keep board-specific raw register constants centralized here so the
//     later GPIO / ADC / TIM1 / BEMF modules can share one mapping source.
//   - These are direct-register bare-metal definitions, not HAL objects.
//   - Pointer constants intentionally stay as raw *mut/*const register
//     addresses because the existing firmware already validates this path.
// ================================================================

// ------------------------------------------------------------
// STM32G4 peripheral base addresses
// ------------------------------------------------------------

pub const RCC_BASE: usize = 0x4002_1000;
pub const GPIOA_BASE: usize = 0x4800_0000;
pub const GPIOB_BASE: usize = 0x4800_0400;
pub const GPIOC_BASE: usize = 0x4800_0800;

pub const ADC1_BASE: usize = 0x5000_0000;
pub const ADC2_BASE: usize = 0x5000_0100;
pub const ADC12_COMMON_BASE: usize = 0x5000_0300;

pub const TIM1_BASE: usize = 0x4001_2C00;

// ------------------------------------------------------------
// RCC registers
// ------------------------------------------------------------

pub const RCC_AHB2ENR: *mut u32 = (RCC_BASE + 0x4C) as *mut u32;
pub const RCC_APB2ENR: *mut u32 = (RCC_BASE + 0x60) as *mut u32;

pub const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;
pub const RCC_AHB2ENR_GPIOBEN: u32 = 1 << 1;
pub const RCC_AHB2ENR_GPIOCEN: u32 = 1 << 2;
pub const RCC_AHB2ENR_ADC12EN: u32 = 1 << 13;

pub const RCC_APB2ENR_TIM1EN: u32 = 1 << 11;

// ------------------------------------------------------------
// GPIO registers
// ------------------------------------------------------------

pub const GPIOA_MODER: *mut u32 = (GPIOA_BASE + 0x00) as *mut u32;
pub const GPIOA_OTYPER: *mut u32 = (GPIOA_BASE + 0x04) as *mut u32;
pub const GPIOA_OSPEEDR: *mut u32 = (GPIOA_BASE + 0x08) as *mut u32;
pub const GPIOA_PUPDR: *mut u32 = (GPIOA_BASE + 0x0C) as *mut u32;
pub const GPIOA_IDR: *const u32 = (GPIOA_BASE + 0x10) as *const u32;
pub const GPIOA_BSRR: *mut u32 = (GPIOA_BASE + 0x18) as *mut u32;
pub const GPIOA_AFRL: *mut u32 = (GPIOA_BASE + 0x20) as *mut u32;
pub const GPIOA_AFRH: *mut u32 = (GPIOA_BASE + 0x24) as *mut u32;
pub const GPIOA_ASCR: *mut u32 = (GPIOA_BASE + 0x2C) as *mut u32;

pub const GPIOB_MODER: *mut u32 = (GPIOB_BASE + 0x00) as *mut u32;
pub const GPIOB_OTYPER: *mut u32 = (GPIOB_BASE + 0x04) as *mut u32;
pub const GPIOB_OSPEEDR: *mut u32 = (GPIOB_BASE + 0x08) as *mut u32;
pub const GPIOB_PUPDR: *mut u32 = (GPIOB_BASE + 0x0C) as *mut u32;
pub const GPIOB_IDR: *const u32 = (GPIOB_BASE + 0x10) as *const u32;
pub const GPIOB_BSRR: *mut u32 = (GPIOB_BASE + 0x18) as *mut u32;
pub const GPIOB_AFRL: *mut u32 = (GPIOB_BASE + 0x20) as *mut u32;
pub const GPIOB_AFRH: *mut u32 = (GPIOB_BASE + 0x24) as *mut u32;
pub const GPIOB_ASCR: *mut u32 = (GPIOB_BASE + 0x2C) as *mut u32;

pub const GPIOC_MODER: *mut u32 = (GPIOC_BASE + 0x00) as *mut u32;
pub const GPIOC_OTYPER: *mut u32 = (GPIOC_BASE + 0x04) as *mut u32;
pub const GPIOC_OSPEEDR: *mut u32 = (GPIOC_BASE + 0x08) as *mut u32;
pub const GPIOC_PUPDR: *mut u32 = (GPIOC_BASE + 0x0C) as *mut u32;
pub const GPIOC_IDR: *const u32 = (GPIOC_BASE + 0x10) as *const u32;
pub const GPIOC_BSRR: *mut u32 = (GPIOC_BASE + 0x18) as *mut u32;
pub const GPIOC_AFRL: *mut u32 = (GPIOC_BASE + 0x20) as *mut u32;
pub const GPIOC_AFRH: *mut u32 = (GPIOC_BASE + 0x24) as *mut u32;
pub const GPIOC_ASCR: *mut u32 = (GPIOC_BASE + 0x2C) as *mut u32;

// ------------------------------------------------------------
// TIM1 registers
// ------------------------------------------------------------

pub const TIM1_CR1: *mut u32 = (TIM1_BASE + 0x00) as *mut u32;
pub const TIM1_CR2: *mut u32 = (TIM1_BASE + 0x04) as *mut u32;
pub const TIM1_EGR: *mut u32 = (TIM1_BASE + 0x14) as *mut u32;
pub const TIM1_CCMR1: *mut u32 = (TIM1_BASE + 0x18) as *mut u32;
pub const TIM1_CCMR2: *mut u32 = (TIM1_BASE + 0x1C) as *mut u32;
pub const TIM1_CCER: *mut u32 = (TIM1_BASE + 0x20) as *mut u32;
pub const TIM1_CNT: *mut u32 = (TIM1_BASE + 0x24) as *mut u32;
pub const TIM1_PSC: *mut u32 = (TIM1_BASE + 0x28) as *mut u32;
pub const TIM1_ARR: *mut u32 = (TIM1_BASE + 0x2C) as *mut u32;
pub const TIM1_RCR: *mut u32 = (TIM1_BASE + 0x30) as *mut u32;
pub const TIM1_CCR1: *mut u32 = (TIM1_BASE + 0x34) as *mut u32;
pub const TIM1_CCR2: *mut u32 = (TIM1_BASE + 0x38) as *mut u32;
pub const TIM1_CCR3: *mut u32 = (TIM1_BASE + 0x3C) as *mut u32;
pub const TIM1_BDTR: *mut u32 = (TIM1_BASE + 0x44) as *mut u32;

pub const TIM1_CR1_CEN: u32 = 1 << 0;
pub const TIM1_CR1_ARPE: u32 = 1 << 7;
pub const TIM1_EGR_UG: u32 = 1 << 0;

pub const TIM1_BDTR_OSSI: u32 = 1 << 10;
pub const TIM1_BDTR_OSSR: u32 = 1 << 11;
pub const TIM1_BDTR_MOE: u32 = 1 << 15;

pub const TIM1_CCMR_FORCED_INACTIVE: u32 = 0b100;
pub const TIM1_CCMR_FORCED_ACTIVE: u32 = 0b101;
pub const TIM1_CCMR_PWM_MODE_1: u32 = 0b110;

// ------------------------------------------------------------
// CCER command states (UNCHANGED commutation table)
// ------------------------------------------------------------

pub const TIM1_CCER_ALL_OFF_FORCED_LOW: u32 =
    (1 << 0) | (1 << 2) | (1 << 3) |
    (1 << 4) | (1 << 6) | (1 << 7) |
    (1 << 8) | (1 << 10) | (1 << 11);

pub const TIM1_CCER_BOOTSTRAP_U_LOW: u32 = (1 << 0) | (1 << 2);
pub const TIM1_CCER_BOOTSTRAP_V_LOW: u32 = (1 << 4) | (1 << 6);
pub const TIM1_CCER_BOOTSTRAP_W_LOW: u32 = (1 << 8) | (1 << 10);

pub const TIM1_CCER_VECTOR_UH_VL: u32 = (1 << 0) | (1 << 4) | (1 << 6);
pub const TIM1_CCER_VECTOR_UH_WL: u32 = (1 << 0) | (1 << 8) | (1 << 10);
pub const TIM1_CCER_VECTOR_VH_WL: u32 = (1 << 4) | (1 << 8) | (1 << 10);
pub const TIM1_CCER_VECTOR_VH_UL: u32 = (1 << 4) | (1 << 0) | (1 << 2);
pub const TIM1_CCER_VECTOR_WH_UL: u32 = (1 << 8) | (1 << 0) | (1 << 2);
pub const TIM1_CCER_VECTOR_WH_VL: u32 = (1 << 8) | (1 << 4) | (1 << 6);

pub const TIM1_TEST_PSC: u32 = 0;
pub const TIM1_TEST_ARR: u32 = 799;

// ------------------------------------------------------------
// Idle / housekeeping timing
// ------------------------------------------------------------

pub const IDLE_LOG_DELAY: u32 = 5_000_000;
pub const IDLE_LED_ON_DELAY: u32 = 250_000;
pub const IDLE_LED_OFF_DELAY: u32 = 2_500_000;

pub const STATE_SETTLE_DELAY: u32 = 150_000;
pub const BOOTSTRAP_HOLD_DELAY: u32 = 400_000;
pub const RELEASE_CHECK_CHUNK: u32 = 40_000;

// ------------------------------------------------------------
// PWM open-loop drive constants
// ------------------------------------------------------------
// Duty is expressed directly as a CCR count against ARR (799).
// 80/800 = 10%, 120/800 = 15%, 160/800 = 20%.

pub const PWM_DUTY_ALIGN: u32 = 80;
pub const PWM_DUTY_RUN_START: u32 = 80;
pub const PWM_DUTY_RUN_MAX: u32 = 100;
pub const PWM_DUTY_INC_PER_EREV: u32 = 5;

pub const ALIGN_HOLD_DELAY: u32 = 1_500_000;

// Ramp for the BEMF-observe build: lower top speed than the ceiling
// test so per-step logging is legible and one electrical rev of log
// shows the BEMF trajectory across all three phases.
// ~150 RPM target: 150000 cyc/vec -> 900000 cyc/erev -> ~17.8 Hz elec
// -> /7 pole-pairs -> ~2.5 rev/s -> ~152 RPM on a 2212.
pub const RAMP_START_VECTOR_DELAY: u32 = 500_000;
pub const RAMP_MIN_VECTOR_DELAY: u32 = 1_000;
pub const RAMP_DECREMENT_PER_ELECTRICAL_REV: u32 = 100_000;
pub const MAX_VECTOR_STEPS_PER_HOLD: u32 = 1000;

pub const TEMP_DELTA_ABORT_RAW: i32 = 500;

// ------------------------------------------------------------
// ADC register helpers
// ------------------------------------------------------------

pub fn adc_isr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x00) as *mut u32
}

pub fn adc_cr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x08) as *mut u32
}

pub fn adc_cfgr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x0C) as *mut u32
}

pub fn adc_smpr1(adc_base: usize) -> *mut u32 {
    (adc_base + 0x14) as *mut u32
}

pub fn adc_smpr2(adc_base: usize) -> *mut u32 {
    (adc_base + 0x18) as *mut u32
}

pub fn adc_sqr1(adc_base: usize) -> *mut u32 {
    (adc_base + 0x30) as *mut u32
}

pub fn adc_dr(adc_base: usize) -> *const u32 {
    (adc_base + 0x40) as *const u32
}

pub fn adc_difsel(adc_base: usize) -> *mut u32 {
    (adc_base + 0xB0) as *mut u32
}

pub const ADC12_CCR: *mut u32 = (ADC12_COMMON_BASE + 0x08) as *mut u32;

pub const ADC_ISR_ADRDY: u32 = 1 << 0;
pub const ADC_ISR_EOC: u32 = 1 << 2;
pub const ADC_ISR_EOS: u32 = 1 << 3;

pub const ADC_CR_ADEN: u32 = 1 << 0;
pub const ADC_CR_ADSTART: u32 = 1 << 2;
pub const ADC_CR_ADVREGEN: u32 = 1 << 28;
pub const ADC_CR_DEEPPWD: u32 = 1 << 29;
pub const ADC_CR_ADCAL: u32 = 1 << 31;

// ------------------------------------------------------------
// Board pins
// ------------------------------------------------------------

pub const STATUS_LED_PIN: u32 = 6;
pub const USER_BUTTON_PIN: u32 = 10;

pub const VBUS_PIN: u32 = 0;
pub const OP1_OUT_PIN: u32 = 2;
pub const OP2_OUT_PIN: u32 = 6;

pub const OP3_OUT_PIN: u32 = 1;
pub const POT_PIN: u32 = 12;
pub const TEMP_PIN: u32 = 14;

pub const DRIVE_UH_PIN: u32 = 8;
pub const DRIVE_VH_PIN: u32 = 9;
pub const DRIVE_WH_PIN: u32 = 10;
pub const DRIVE_VL_PIN: u32 = 12;
pub const DRIVE_WL_PIN: u32 = 15;
pub const DRIVE_UL_PIN: u32 = 13;

pub const AF_TIM1_PA: u32 = 6;
pub const AF_TIM1_N: u32 = 4;

pub const VBUS_ADC_CHANNEL: u32 = 1;
pub const OP1_OUT_ADC_CHANNEL: u32 = 3;
pub const OP2_OUT_ADC_CHANNEL: u32 = 3;
pub const OP3_OUT_ADC_CHANNEL: u32 = 12;
pub const TEMP_ADC_CHANNEL: u32 = 5;
pub const POT_ADC_CHANNEL: u32 = 11;

pub const ADC_TIMEOUT_VALUE: u16 = 0xFFFF;

// ------------------------------------------------------------
// BEMF sense (observe-only) — B-G431B-ESC1 "BEMF DETECTION" network
// ------------------------------------------------------------
// GPIO_BEMF (PB5): output-LOW enables the divider (PWM-ON sampling);
// INPUT / high-Z disables it (PWM-OFF, ground-referenced sampling).
// At our low duty we use PWM-OFF sampling -> PB5 held as INPUT.
//
// Phase -> tap -> pin -> ADC2 channel (confirm vs DS12589 on first run):
//   U / OUT1 -> BEMF1 -> PA4  -> ADC2_IN17
//   V / OUT2 -> BEMF2 -> PC4  -> ADC2_IN5
//   W / OUT3 -> BEMF3 -> PB11 -> ADC2_IN14

pub const GPIO_BEMF_PIN: u32 = 5; // PB5

pub const BEMF1_PIN: u32 = 4; // PA4
pub const BEMF2_PIN: u32 = 4; // PC4
pub const BEMF3_PIN: u32 = 11; // PB11

pub const BEMF1_ADC_CHANNEL: u32 = 17; // ADC2_IN17 (PA4, U)
pub const BEMF2_ADC_CHANNEL: u32 = 5; // ADC2_IN5  (PC4, V)
pub const BEMF3_ADC_CHANNEL: u32 = 14; // ADC2_IN14 (PB11, W)

pub const BEMF_SAMPLE_BITS: u32 = 0b011; // 24.5 ADC cycles (fast, fits off-window)
pub const BEMF_BLANK_DELAY: u32 = 30_000; // post-commutation demag before sampling
pub const BEMF_SAMPLES_PER_STEP: usize = 4;
// ================================================================
// Footer
// File: regs.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/regs.rs
// Version: v0.4.1-split-regs-same-behavior
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================
