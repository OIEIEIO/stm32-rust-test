// ================================================================
// File: regs.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/regs.rs
// Version: v0.5.24-warning-cleanup
// Purpose: Register addresses, bit masks, board pin IDs, ADC channel IDs,
//          OPAMP current-sense bring-up constants, OP3/VOPAMP3
//          diagnostic constants, ADC injected sampling constants, PWM
//          constants, sine/SPWM bring-up constants, and small raw-register
//          address helpers for the B-G431B-ESC1 STM32G431CB firmware.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.5.23:
//   - Removes unused diagnostic constants that were kept from earlier
//     OPAMP, TIM1_CH4, and ADC injected-debug bring-up passes.
//   - Keeps only constants used by the current v0.5.23/v0.5.24 runtime
//     model.
//   - No register values used by the active firmware path are changed.
//   - No PWM, TIM1_CH4 timing, sine ramp, injected ADC, regular ADC,
//     OPAMP, or drive behavior change.
//
// Change summary vs v0.5.22:
//   - Adds OP3/VOPAMP3 diagnostic constants for proving the internal
//     OPAMP3-to-ADC2 path without changing the default motor-drive path.
//   - Keeps the existing default injected pair as ADC1=OP1 external VOUT
//     and ADC2=OP2 external VOUT.
//   - Adds a separate ADC2 channel constant for VOPAMP3 internal routing
//     so the next pass can log OP3 internal output beside the external
//     PB1/ADC1_IN12 observation.
//   - No PWM, TIM1_CH4 timing, sine ramp, or six-step behavior change.
//
// Learning notes:
//   - Six-step mode uses one PWM high side, one solid low side, and one
//     floating phase. Sine/SPWM mode drives all three half-bridges with
//     complementary PWM, so TIM1 hardware dead-time matters.
//   - TIM1 CH1/CH1N, CH2/CH2N, and CH3/CH3N are treated as three
//     complementary half-bridges. CCER enables the main and N outputs;
//     BDTR.MOE gates the bridge globally.
//   - Sine bring-up constants are deliberately conservative. The first
//     goal is to prove smooth three-phase PWM generation under the same
//     dead-man-button safety model, not to produce torque efficiently.
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

// OPAMP register block. STM32G431 OPAMP CSR registers are contiguous
// 32-bit registers inside the OPAMP control block.
//
//   OPAMP1_CSR: OPAMP_BASE + 0x00
//   OPAMP2_CSR: OPAMP_BASE + 0x04
//   OPAMP3_CSR: OPAMP_BASE + 0x08
pub const OPAMP_BASE: usize = 0x4001_0300;

pub const OPAMP1_CSR: *mut u32 = (OPAMP_BASE + 0x00) as *mut u32;
pub const OPAMP2_CSR: *mut u32 = (OPAMP_BASE + 0x04) as *mut u32;
pub const OPAMP3_CSR: *mut u32 = (OPAMP_BASE + 0x08) as *mut u32;

// ------------------------------------------------------------
// RCC registers
// ------------------------------------------------------------

pub const RCC_AHB2ENR: *mut u32 = (RCC_BASE + 0x4C) as *mut u32;
pub const RCC_APB2ENR: *mut u32 = (RCC_BASE + 0x60) as *mut u32;

pub const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;
pub const RCC_AHB2ENR_GPIOBEN: u32 = 1 << 1;
pub const RCC_AHB2ENR_GPIOCEN: u32 = 1 << 2;
pub const RCC_AHB2ENR_ADC12EN: u32 = 1 << 13;

pub const RCC_APB2ENR_SYSCFGEN: u32 = 1 << 0;
pub const RCC_APB2ENR_TIM1EN: u32 = 1 << 11;

// ------------------------------------------------------------
// OPAMP CSR bits: STM32G4 current-sense bring-up
// ------------------------------------------------------------
// Current active model:
//   - OPAMP1/2: external VOUT ADC path.
//   - OPAMP3: external VOUT ADC path plus internal VOPAMP3 diagnostic.
//   - OPAMP3/VOPAMP3 is diagnostic only, not phase-current reconstruction.

pub const OPAMP_CSR_OPAMPXEN: u32 = 1 << 0;
pub const OPAMP_CSR_VPSEL_IO0: u32 = 0b00 << 2;
pub const OPAMP_CSR_VMSEL_PGA: u32 = 0b10 << 5;
pub const OPAMP_CSR_HIGHSPEEDEN: u32 = 1 << 7;
pub const OPAMP_CSR_OPAMPINTEN: u32 = 1 << 8;
pub const OPAMP_CSR_PGGAIN_X16_OR_MINUS_15: u32 = (1 << 15) | (1 << 14);

// OPAMP1/2 external VOUT configuration.
//   OP1 VOUT PA2 -> ADC1_IN3
//   OP2 VOUT PA6 -> ADC2_IN3
//   OP3 VOUT PB1 -> ADC1_IN12
pub const OPAMP_CURRENT_SENSE_CONFIG: u32 =
    OPAMP_CSR_VPSEL_IO0
        | OPAMP_CSR_VMSEL_PGA
        | OPAMP_CSR_PGGAIN_X16_OR_MINUS_15
        | OPAMP_CSR_HIGHSPEEDEN;

// OPAMP3 diagnostic configuration: same PGA/current-sense setup, plus
// internal output routing. OPAMP1/2 remain on the external VOUT path.
pub const OPAMP3_VOPAMP3_DIAG_CONFIG: u32 =
    OPAMP_CURRENT_SENSE_CONFIG | OPAMP_CSR_OPAMPINTEN;

pub const CURRENT_SENSE_OFFSET_SAMPLES: u32 = 64;
pub const CURRENT_SENSE_OFFSET_SAMPLE_DELAY: u32 = 10_000;
pub const CURRENT_SENSE_PRE_OFFSET_SETTLE_DELAY: u32 = 800_000;
pub const CURRENT_SENSE_DUMMY_READS_BEFORE_OFFSET: u32 = 16;
pub const CURRENT_SENSE_NEAR_HIGH_RAIL_RAW: u16 = 4080;

// Current-sense ADC sample-time field used by OPAMP output channels.
pub const CURRENT_SENSE_SYNC_SAMPLE_BITS: u32 = 0b011; // 24.5 ADC cycles
pub const CURRENT_SENSE_SYNC_TARGET_CNT: u32 = TIM1_TEST_ARR / 2;

// TIM1_CH4-triggered injected ADC path.
// Active injected pair:
//   - ADC1 injected rank 1 = OPAMP1 external VOUT
//   - ADC2 injected rank 1 = OPAMP2 external VOUT
pub const CURRENT_SENSE_INJECTED_TIM1_CCR4: u32 = CURRENT_SENSE_SYNC_TARGET_CNT;
pub const CURRENT_SENSE_INJECTED_WAIT_MAX_LOOPS: u32 = 4_000;

// OP3/VOPAMP3 regular ADC diagnostic path.
// External OP3 remains PB1 -> ADC1_IN12.
// Internal VOPAMP3 path is sampled as ADC2 channel 18.
pub const OP3_VOPAMP3_ADC2_CHANNEL: u32 = 18;

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
pub const TIM1_CCR4: *mut u32 = (TIM1_BASE + 0x40) as *mut u32;
pub const TIM1_BDTR: *mut u32 = (TIM1_BASE + 0x44) as *mut u32;

pub const TIM1_CR1_CEN: u32 = 1 << 0;
pub const TIM1_CR1_ARPE: u32 = 1 << 7;
pub const TIM1_EGR_UG: u32 = 1 << 0;

pub const TIM1_BDTR_DTG_MASK: u32 = 0xFF;
pub const TIM1_BDTR_OSSI: u32 = 1 << 10;
pub const TIM1_BDTR_OSSR: u32 = 1 << 11;
pub const TIM1_BDTR_MOE: u32 = 1 << 15;

pub const TIM1_CCMR_FORCED_INACTIVE: u32 = 0b100;
pub const TIM1_CCMR_FORCED_ACTIVE: u32 = 0b101;
pub const TIM1_CCMR_PWM_MODE_1: u32 = 0b110;

// TIM1_CCMR2 channel-4 output-compare field definitions.
// CH4 is not routed to a board pin here; it is used only as an
// internal compare/reference event for current-sense ADC trigger debug.
pub const TIM1_CCMR2_CC4S_MASK: u32 = 0b11 << 8;
pub const TIM1_CCMR2_OC4FE: u32 = 1 << 10;
pub const TIM1_CCMR2_OC4PE: u32 = 1 << 11;
pub const TIM1_CCMR2_OC4M_MASK: u32 = 0b111 << 12;
pub const TIM1_CCMR2_OC4M_PWM_MODE_2: u32 = 0b111 << 12;
pub const TIM1_CCMR2_OC4_INTERNAL_TRIGGER_CONFIG: u32 = TIM1_CCMR2_OC4M_PWM_MODE_2;

pub const TIM1_CCER_CC4E: u32 = 1 << 12;
pub const TIM1_CCER_CC4P: u32 = 1 << 13;
pub const TIM1_CCER_CC4NE: u32 = 1 << 14;
pub const TIM1_CCER_CC4NP: u32 = 1 << 15;
pub const TIM1_CCER_CC4_OUTPUT_MASK: u32 =
    TIM1_CCER_CC4E | TIM1_CCER_CC4P | TIM1_CCER_CC4NE | TIM1_CCER_CC4NP;

// ------------------------------------------------------------
// CCER command states: six-step baseline
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

// ------------------------------------------------------------
// CCER / BDTR command states: sine/SPWM bring-up
// ------------------------------------------------------------
// Enables CH1, CH1N, CH2, CH2N, CH3, CH3N with default polarity.

pub const TIM1_CCER_SINE_COMPLEMENTARY_PWM: u32 =
    (1 << 0) | (1 << 2) |
    (1 << 4) | (1 << 6) |
    (1 << 8) | (1 << 10);

// DTG=32 at a 16 MHz timer clock is about 2 us.
pub const TIM1_BDTR_SINE_SAFE_DTG: u32 = 32;

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
// PWM open-loop six-step drive constants
// ------------------------------------------------------------

pub const PWM_DUTY_ALIGN: u32 = 80;
pub const PWM_DUTY_RUN_START: u32 = 80;
pub const PWM_DUTY_RUN_MAX: u32 = 100;
pub const PWM_DUTY_INC_PER_EREV: u32 = 5;

pub const ALIGN_HOLD_DELAY: u32 = 1_500_000;

pub const RAMP_START_VECTOR_DELAY: u32 = 500_000;
pub const RAMP_MIN_VECTOR_DELAY: u32 = 1_000;
pub const RAMP_DECREMENT_PER_ELECTRICAL_REV: u32 = 100_000;
pub const MAX_VECTOR_STEPS_PER_HOLD: u32 = 1000;

pub const TEMP_DELTA_ABORT_RAW: i32 = 500;

// ------------------------------------------------------------
// Open-loop sine/SPWM first-test constants
// ------------------------------------------------------------

pub const SINE_PWM_CENTER: u32 = TIM1_TEST_ARR / 2;
pub const SINE_PWM_MIN_DUTY: u32 = 40;
pub const SINE_PWM_MAX_DUTY: u32 = TIM1_TEST_ARR - 40;

pub const SINE_PWM_START_AMPLITUDE: u32 = 45;
pub const SINE_PWM_RUN_MAX_AMPLITUDE: u32 = 100;
pub const SINE_PWM_INC_PER_ELECTRICAL_REV: u32 = 4;

pub const SINE_TABLE_LEN: usize = 96;
pub const SINE_ALIGN_HOLD_DELAY: u32 = 1_000_000;

pub const SINE_START_STEP_DELAY: u32 = 8_000;
pub const SINE_MIN_STEP_DELAY: u32 = 2_850;
pub const SINE_DECREMENT_PER_ELECTRICAL_REV: u32 = 250;
pub const SINE_MAX_STEPS_PER_HOLD: u32 = 32000;
pub const SINE_LOG_EVERY_STEPS: u32 = 960;

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

pub fn adc_jsqr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x4C) as *mut u32
}

pub fn adc_jdr1(adc_base: usize) -> *const u32 {
    (adc_base + 0x80) as *const u32
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
pub const ADC_ISR_JEOC: u32 = 1 << 5;
pub const ADC_ISR_JEOS: u32 = 1 << 6;
pub const ADC_ISR_JQOVF: u32 = 1 << 10;

pub const ADC_CR_ADEN: u32 = 1 << 0;
pub const ADC_CR_ADSTART: u32 = 1 << 2;
pub const ADC_CR_JADSTART: u32 = 1 << 3;
pub const ADC_CR_JADSTP: u32 = 1 << 5;
pub const ADC_CR_ADVREGEN: u32 = 1 << 28;
pub const ADC_CR_DEEPPWD: u32 = 1 << 29;
pub const ADC_CR_ADCAL: u32 = 1 << 31;

pub const ADC_CFGR_JQDIS: u32 = 1 << 31;

pub const ADC_JSQR_JEXTSEL_TIM1_CH4: u32 = 0b0001 << 2;
pub const ADC_JSQR_JEXTEN_RISING: u32 = 0b01 << 6;

// STM32G4 JSQR rank channel fields are 5-bit fields. Rank 1 starts at
// bit 9, not bit 8.
pub const ADC_JSQR_JSQ1_SHIFT: u32 = 9;

// ------------------------------------------------------------
// Board pins
// ------------------------------------------------------------

pub const STATUS_LED_PIN: u32 = 6;
pub const USER_BUTTON_PIN: u32 = 10;

pub const VBUS_PIN: u32 = 0;

// Current-sense OPAMP positive inputs and outputs.
// Port A: OPAMP1 VINP0 PA1, OPAMP1 VOUT PA2,
//         OPAMP2 VOUT PA6, OPAMP2 VINP0 PA7.
pub const OP1_INP_PIN: u32 = 1;
pub const OP1_OUT_PIN: u32 = 2;
pub const OP2_OUT_PIN: u32 = 6;
pub const OP2_INP_PIN: u32 = 7;

// Port B: OPAMP3 VINP0 PB0, OPAMP3 VOUT PB1.
pub const OP3_INP_PIN: u32 = 0;
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
pub const OP3_INTERNAL_VOPAMP3_ADC2_CHANNEL: u32 = OP3_VOPAMP3_ADC2_CHANNEL;
pub const TEMP_ADC_CHANNEL: u32 = 5;
pub const POT_ADC_CHANNEL: u32 = 11;

pub const ADC_TIMEOUT_VALUE: u16 = 0xFFFF;

// ------------------------------------------------------------
// BEMF sense observe-only network
// ------------------------------------------------------------
// GPIO_BEMF PB5:
//   - output low enables divider for PWM-on sampling.
//   - input/high-Z disables divider for PWM-off, ground-referenced sampling.
//
// Current code uses PWM-off sampling -> PB5 held as input.

pub const GPIO_BEMF_PIN: u32 = 5; // PB5

pub const BEMF1_PIN: u32 = 4; // PA4
pub const BEMF2_PIN: u32 = 4; // PC4
pub const BEMF3_PIN: u32 = 11; // PB11

pub const BEMF1_ADC_CHANNEL: u32 = 17; // ADC2_IN17, U
pub const BEMF2_ADC_CHANNEL: u32 = 5;  // ADC2_IN5,  V
pub const BEMF3_ADC_CHANNEL: u32 = 14; // ADC2_IN14, W

pub const BEMF_SAMPLE_BITS: u32 = 0b011; // 24.5 ADC cycles
pub const BEMF_BLANK_DELAY: u32 = 30_000;
pub const BEMF_SAMPLES_PER_STEP: usize = 4;

// ================================================================
// Footer
// File: regs.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/regs.rs
// Version: v0.5.24-warning-cleanup
// Created: 2026-06-08
// Generated timestamp: 2026-06-08T17:10:00Z
// ================================================================