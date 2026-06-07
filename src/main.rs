// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.2.1-tim1-internal-counter-moe-off
// Purpose: STM32G431CB Rust bring-up: configure TIM1 internally while keeping all L6387 drive inputs GPIO LOW
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
// ================================================================

#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};

use cortex_m::asm;
use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

// ------------------------------------------------------------
// STM32G4 peripheral base addresses
// ------------------------------------------------------------

const RCC_BASE: usize = 0x4002_1000;
const GPIOA_BASE: usize = 0x4800_0000;
const GPIOB_BASE: usize = 0x4800_0400;
const GPIOC_BASE: usize = 0x4800_0800;

const ADC1_BASE: usize = 0x5000_0000;
const ADC2_BASE: usize = 0x5000_0100;
const ADC12_COMMON_BASE: usize = 0x5000_0300;

const TIM1_BASE: usize = 0x4001_2C00;

// ------------------------------------------------------------
// RCC registers
// ------------------------------------------------------------

const RCC_AHB2ENR: *mut u32 = (RCC_BASE + 0x4C) as *mut u32;
const RCC_APB2ENR: *mut u32 = (RCC_BASE + 0x60) as *mut u32;

const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;
const RCC_AHB2ENR_GPIOBEN: u32 = 1 << 1;
const RCC_AHB2ENR_GPIOCEN: u32 = 1 << 2;
const RCC_AHB2ENR_ADC12EN: u32 = 1 << 13;

const RCC_APB2ENR_TIM1EN: u32 = 1 << 11;

// ------------------------------------------------------------
// GPIO registers
// ------------------------------------------------------------

const GPIOA_MODER: *mut u32 = (GPIOA_BASE + 0x00) as *mut u32;
const GPIOA_OTYPER: *mut u32 = (GPIOA_BASE + 0x04) as *mut u32;
const GPIOA_OSPEEDR: *mut u32 = (GPIOA_BASE + 0x08) as *mut u32;
const GPIOA_PUPDR: *mut u32 = (GPIOA_BASE + 0x0C) as *mut u32;
const GPIOA_IDR: *const u32 = (GPIOA_BASE + 0x10) as *const u32;
const GPIOA_BSRR: *mut u32 = (GPIOA_BASE + 0x18) as *mut u32;
const GPIOA_ASCR: *mut u32 = (GPIOA_BASE + 0x2C) as *mut u32;

const GPIOB_MODER: *mut u32 = (GPIOB_BASE + 0x00) as *mut u32;
const GPIOB_OTYPER: *mut u32 = (GPIOB_BASE + 0x04) as *mut u32;
const GPIOB_OSPEEDR: *mut u32 = (GPIOB_BASE + 0x08) as *mut u32;
const GPIOB_PUPDR: *mut u32 = (GPIOB_BASE + 0x0C) as *mut u32;
const GPIOB_IDR: *const u32 = (GPIOB_BASE + 0x10) as *const u32;
const GPIOB_BSRR: *mut u32 = (GPIOB_BASE + 0x18) as *mut u32;
const GPIOB_ASCR: *mut u32 = (GPIOB_BASE + 0x2C) as *mut u32;

const GPIOC_MODER: *mut u32 = (GPIOC_BASE + 0x00) as *mut u32;
const GPIOC_OTYPER: *mut u32 = (GPIOC_BASE + 0x04) as *mut u32;
const GPIOC_OSPEEDR: *mut u32 = (GPIOC_BASE + 0x08) as *mut u32;
const GPIOC_PUPDR: *mut u32 = (GPIOC_BASE + 0x0C) as *mut u32;
const GPIOC_IDR: *const u32 = (GPIOC_BASE + 0x10) as *const u32;
const GPIOC_BSRR: *mut u32 = (GPIOC_BASE + 0x18) as *mut u32;

// ------------------------------------------------------------
// TIM1 registers
// ------------------------------------------------------------

const TIM1_CR1: *mut u32 = (TIM1_BASE + 0x00) as *mut u32;
const TIM1_EGR: *mut u32 = (TIM1_BASE + 0x14) as *mut u32;
const TIM1_CCMR1: *mut u32 = (TIM1_BASE + 0x18) as *mut u32;
const TIM1_CCMR2: *mut u32 = (TIM1_BASE + 0x1C) as *mut u32;
const TIM1_CCER: *mut u32 = (TIM1_BASE + 0x20) as *mut u32;
const TIM1_CNT: *mut u32 = (TIM1_BASE + 0x24) as *mut u32;
const TIM1_PSC: *mut u32 = (TIM1_BASE + 0x28) as *mut u32;
const TIM1_ARR: *mut u32 = (TIM1_BASE + 0x2C) as *mut u32;
const TIM1_RCR: *mut u32 = (TIM1_BASE + 0x30) as *mut u32;
const TIM1_CCR1: *mut u32 = (TIM1_BASE + 0x34) as *mut u32;
const TIM1_CCR2: *mut u32 = (TIM1_BASE + 0x38) as *mut u32;
const TIM1_CCR3: *mut u32 = (TIM1_BASE + 0x3C) as *mut u32;
const TIM1_BDTR: *mut u32 = (TIM1_BASE + 0x44) as *mut u32;

const TIM1_CR1_CEN: u32 = 1 << 0;
const TIM1_CR1_ARPE: u32 = 1 << 7;
const TIM1_EGR_UG: u32 = 1 << 0;
const TIM1_BDTR_MOE: u32 = 1 << 15;

const TIM1_TEST_PSC: u32 = 0;
const TIM1_TEST_ARR: u32 = 3999;

// ------------------------------------------------------------
// ADC register helpers
// ------------------------------------------------------------

fn adc_isr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x00) as *mut u32
}

fn adc_cr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x08) as *mut u32
}

fn adc_cfgr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x0C) as *mut u32
}

fn adc_smpr1(adc_base: usize) -> *mut u32 {
    (adc_base + 0x14) as *mut u32
}

fn adc_smpr2(adc_base: usize) -> *mut u32 {
    (adc_base + 0x18) as *mut u32
}

fn adc_sqr1(adc_base: usize) -> *mut u32 {
    (adc_base + 0x30) as *mut u32
}

fn adc_dr(adc_base: usize) -> *const u32 {
    (adc_base + 0x40) as *const u32
}

fn adc_difsel(adc_base: usize) -> *mut u32 {
    (adc_base + 0xB0) as *mut u32
}

const ADC12_CCR: *mut u32 = (ADC12_COMMON_BASE + 0x08) as *mut u32;

// ADC ISR bits
const ADC_ISR_ADRDY: u32 = 1 << 0;
const ADC_ISR_EOC: u32 = 1 << 2;
const ADC_ISR_EOS: u32 = 1 << 3;

// ADC CR bits
const ADC_CR_ADEN: u32 = 1 << 0;
const ADC_CR_ADSTART: u32 = 1 << 2;
const ADC_CR_ADVREGEN: u32 = 1 << 28;
const ADC_CR_DEEPPWD: u32 = 1 << 29;
const ADC_CR_ADCAL: u32 = 1 << 31;

// ------------------------------------------------------------
// Board pins
// ------------------------------------------------------------

const STATUS_LED_PIN: u32 = 6;   // PC6
const USER_BUTTON_PIN: u32 = 10; // PC10

const VBUS_PIN: u32 = 0;    // PA0
const OP1_OUT_PIN: u32 = 2; // PA2
const OP2_OUT_PIN: u32 = 6; // PA6

const OP3_OUT_PIN: u32 = 1; // PB1
const POT_PIN: u32 = 12;    // PB12
const TEMP_PIN: u32 = 14;   // PB14

// L6387 drive input pins.
// These remain GPIO outputs LOW in this version.
// TIM1 is configured internally only.
const DRIVE_UH_PIN: u32 = 8;  // PA8  / TIM1_CH1  / UH
const DRIVE_VH_PIN: u32 = 9;  // PA9  / TIM1_CH2  / VH
const DRIVE_WH_PIN: u32 = 10; // PA10 / TIM1_CH3  / WH
const DRIVE_VL_PIN: u32 = 12; // PA12 / TIM1_CH2N / VL
const DRIVE_WL_PIN: u32 = 15; // PB15 / TIM1_CH3N / WL
const DRIVE_UL_PIN: u32 = 13; // PC13 / TIM1_CH1N / UL

// ADC channels
const VBUS_ADC_CHANNEL: u32 = 1;     // PA0  / ADC1_IN1
const OP1_OUT_ADC_CHANNEL: u32 = 3;  // PA2  / ADC1_IN3
const OP2_OUT_ADC_CHANNEL: u32 = 3;  // PA6  / ADC2_IN3
const OP3_OUT_ADC_CHANNEL: u32 = 12; // PB1  / ADC1_IN12
const TEMP_ADC_CHANNEL: u32 = 5;     // PB14 / ADC1_IN5
const POT_ADC_CHANNEL: u32 = 11;     // PB12 / ADC1_IN11

const ADC_TIMEOUT_VALUE: u16 = 0xFFFF;

// ------------------------------------------------------------
// Drive-pin and TIM1 readback
// ------------------------------------------------------------

#[derive(Copy, Clone)]
struct DriveReadback {
    uh: u32,
    ul: u32,
    vh: u32,
    vl: u32,
    wh: u32,
    wl: u32,
    safe_low: u32,
}

#[derive(Copy, Clone)]
struct Tim1Readback {
    cnt_a: u32,
    cnt_b: u32,
    counting: u32,
    psc: u32,
    arr: u32,
    ccr1: u32,
    ccr2: u32,
    ccr3: u32,
    ccer: u32,
    moe: u32,
}

fn read_pin(idr: *const u32, pin: u32) -> u32 {
    let value = unsafe { read_volatile(idr) };

    if (value & (1 << pin)) != 0 {
        1
    } else {
        0
    }
}

fn force_drive_pins_low() {
    unsafe {
        // Reset GPIOA drive pins: UH, VH, WH, VL.
        write_volatile(
            GPIOA_BSRR,
            (1 << (DRIVE_UH_PIN + 16))
                | (1 << (DRIVE_VH_PIN + 16))
                | (1 << (DRIVE_WH_PIN + 16))
                | (1 << (DRIVE_VL_PIN + 16)),
        );

        // Reset GPIOB drive pin: WL.
        write_volatile(GPIOB_BSRR, 1 << (DRIVE_WL_PIN + 16));

        // Reset GPIOC drive pin: UL.
        write_volatile(GPIOC_BSRR, 1 << (DRIVE_UL_PIN + 16));
    }
}

fn read_drive_pins() -> DriveReadback {
    let uh = read_pin(GPIOA_IDR, DRIVE_UH_PIN);
    let vh = read_pin(GPIOA_IDR, DRIVE_VH_PIN);
    let wh = read_pin(GPIOA_IDR, DRIVE_WH_PIN);
    let vl = read_pin(GPIOA_IDR, DRIVE_VL_PIN);
    let wl = read_pin(GPIOB_IDR, DRIVE_WL_PIN);
    let ul = read_pin(GPIOC_IDR, DRIVE_UL_PIN);

    let safe_low = if uh == 0 && ul == 0 && vh == 0 && vl == 0 && wh == 0 && wl == 0 {
        1
    } else {
        0
    };

    DriveReadback {
        uh,
        ul,
        vh,
        vl,
        wh,
        wl,
        safe_low,
    }
}

fn read_tim1() -> Tim1Readback {
    unsafe {
        let cnt_a = read_volatile(TIM1_CNT);
        asm::delay(16_000);
        let cnt_b = read_volatile(TIM1_CNT);

        let psc = read_volatile(TIM1_PSC);
        let arr = read_volatile(TIM1_ARR);
        let ccr1 = read_volatile(TIM1_CCR1);
        let ccr2 = read_volatile(TIM1_CCR2);
        let ccr3 = read_volatile(TIM1_CCR3);
        let ccer = read_volatile(TIM1_CCER);
        let bdtr = read_volatile(TIM1_BDTR);

        let counting = if cnt_a != cnt_b { 1 } else { 0 };
        let moe = if (bdtr & TIM1_BDTR_MOE) != 0 { 1 } else { 0 };

        Tim1Readback {
            cnt_a,
            cnt_b,
            counting,
            psc,
            arr,
            ccr1,
            ccr2,
            ccr3,
            ccer,
            moe,
        }
    }
}

// ------------------------------------------------------------
// GPIO helpers
// ------------------------------------------------------------

fn led_high() {
    unsafe {
        write_volatile(GPIOC_BSRR, 1 << STATUS_LED_PIN);
    }
}

fn led_low() {
    unsafe {
        write_volatile(GPIOC_BSRR, 1 << (STATUS_LED_PIN + 16));
    }
}

fn button_pressed() -> bool {
    let idr = unsafe { read_volatile(GPIOC_IDR) };

    // Confirmed from v0.1.2:
    // PC10 button is active-low with pull-up.
    (idr & (1 << USER_BUTTON_PIN)) == 0
}

// ------------------------------------------------------------
// Delay helpers
// ------------------------------------------------------------

fn delay_cycles(cycles: u32) {
    asm::delay(cycles);
}

fn delay_fast_button() {
    asm::delay(650_000);
}

// ------------------------------------------------------------
// ADC helpers
// ------------------------------------------------------------

fn adc_select_channel(adc_base: usize, channel: u32) {
    unsafe {
        // Regular sequence length = 1 conversion.
        // SQ1 = selected channel, bits [10:6].
        let sqr1 = channel << 6;
        write_volatile(adc_sqr1(adc_base), sqr1);
    }
}

fn adc_set_sample_time(adc_base: usize, channel: u32) {
    unsafe {
        // Use longest sampling time for all bring-up channels.
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

fn adc_set_single_ended(adc_base: usize, channel: u32) {
    unsafe {
        let mut difsel = read_volatile(adc_difsel(adc_base));
        difsel &= !(1 << channel);
        write_volatile(adc_difsel(adc_base), difsel);
    }
}

fn adc_read_channel_raw(adc_base: usize, channel: u32) -> u16 {
    unsafe {
        adc_select_channel(adc_base, channel);

        // Clear old EOC/EOS flags.
        write_volatile(adc_isr(adc_base), ADC_ISR_EOC | ADC_ISR_EOS);

        // Start regular conversion.
        let cr = read_volatile(adc_cr(adc_base));
        write_volatile(adc_cr(adc_base), cr | ADC_CR_ADSTART);

        // Poll for conversion complete with a timeout.
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

fn adc_live_percent(raw: u16) -> u32 {
    let clamped_raw = if raw > 4095 { 2048 } else { raw };
    ((clamped_raw as u32) * 100) / 4095
}

fn adc_delta(current: u16, baseline: u16) -> i32 {
    (current as i32) - (baseline as i32)
}

fn pot_to_delay(raw: u16) -> u32 {
    let clamped_raw = if raw > 4095 { 2048 } else { raw };
    let raw_u64 = clamped_raw as u64;

    let fast_delay: u64 = 650_000;
    let slow_delay: u64 = 4_000_000;
    let span: u64 = slow_delay - fast_delay;

    let delay = slow_delay - ((raw_u64 * span) / 4095);

    delay as u32
}

fn blink_with_delay(delay: u32) {
    led_high();
    delay_cycles(delay);

    led_low();
    delay_cycles(delay);
}

fn blink_fast_button_override() {
    led_high();
    delay_fast_button();

    led_low();
    delay_fast_button();
}

// ------------------------------------------------------------
// Peripheral setup
// ------------------------------------------------------------

fn setup_gpio() {
    unsafe {
        // Enable GPIOA, GPIOB, GPIOC, and ADC12 clocks.
        let rcc_ahb2enr = read_volatile(RCC_AHB2ENR);
        write_volatile(
            RCC_AHB2ENR,
            rcc_ahb2enr
                | RCC_AHB2ENR_GPIOAEN
                | RCC_AHB2ENR_GPIOBEN
                | RCC_AHB2ENR_GPIOCEN
                | RCC_AHB2ENR_ADC12EN,
        );

        asm::delay(8_000);

        // Preload all drive output latches LOW before changing MODER to output.
        force_drive_pins_low();

        // GPIOA:
        // PA0  = VBUS analog
        // PA2  = OP1_OUT analog
        // PA6  = OP2_OUT analog
        // PA8  = UH GPIO output LOW
        // PA9  = VH GPIO output LOW
        // PA10 = WH GPIO output LOW
        // PA12 = VL GPIO output LOW
        let mut gpioa_moder = read_volatile(GPIOA_MODER);

        gpioa_moder &= !(0b11 << (VBUS_PIN * 2));
        gpioa_moder |= 0b11 << (VBUS_PIN * 2);

        gpioa_moder &= !(0b11 << (OP1_OUT_PIN * 2));
        gpioa_moder |= 0b11 << (OP1_OUT_PIN * 2);

        gpioa_moder &= !(0b11 << (OP2_OUT_PIN * 2));
        gpioa_moder |= 0b11 << (OP2_OUT_PIN * 2);

        gpioa_moder &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_moder |= 0b01 << (DRIVE_UH_PIN * 2);

        gpioa_moder &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_moder |= 0b01 << (DRIVE_VH_PIN * 2);

        gpioa_moder &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_moder |= 0b01 << (DRIVE_WH_PIN * 2);

        gpioa_moder &= !(0b11 << (DRIVE_VL_PIN * 2));
        gpioa_moder |= 0b01 << (DRIVE_VL_PIN * 2);

        write_volatile(GPIOA_MODER, gpioa_moder);

        let mut gpioa_otyper = read_volatile(GPIOA_OTYPER);
        gpioa_otyper &= !(1 << DRIVE_UH_PIN);
        gpioa_otyper &= !(1 << DRIVE_VH_PIN);
        gpioa_otyper &= !(1 << DRIVE_WH_PIN);
        gpioa_otyper &= !(1 << DRIVE_VL_PIN);
        write_volatile(GPIOA_OTYPER, gpioa_otyper);

        let mut gpioa_ospeedr = read_volatile(GPIOA_OSPEEDR);
        gpioa_ospeedr &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_VL_PIN * 2));
        write_volatile(GPIOA_OSPEEDR, gpioa_ospeedr);

        let mut gpioa_pupdr = read_volatile(GPIOA_PUPDR);
        gpioa_pupdr &= !(0b11 << (VBUS_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP1_OUT_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP2_OUT_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_VL_PIN * 2));
        write_volatile(GPIOA_PUPDR, gpioa_pupdr);

        let mut gpioa_ascr = read_volatile(GPIOA_ASCR);
        gpioa_ascr |= 1 << VBUS_PIN;
        gpioa_ascr |= 1 << OP1_OUT_PIN;
        gpioa_ascr |= 1 << OP2_OUT_PIN;
        write_volatile(GPIOA_ASCR, gpioa_ascr);

        // GPIOB:
        // PB1  = OP3_OUT analog
        // PB12 = potentiometer analog
        // PB14 = temperature feedback analog
        // PB15 = WL GPIO output LOW
        let mut gpiob_moder = read_volatile(GPIOB_MODER);

        gpiob_moder &= !(0b11 << (OP3_OUT_PIN * 2));
        gpiob_moder |= 0b11 << (OP3_OUT_PIN * 2);

        gpiob_moder &= !(0b11 << (POT_PIN * 2));
        gpiob_moder |= 0b11 << (POT_PIN * 2);

        gpiob_moder &= !(0b11 << (TEMP_PIN * 2));
        gpiob_moder |= 0b11 << (TEMP_PIN * 2);

        gpiob_moder &= !(0b11 << (DRIVE_WL_PIN * 2));
        gpiob_moder |= 0b01 << (DRIVE_WL_PIN * 2);

        write_volatile(GPIOB_MODER, gpiob_moder);

        let mut gpiob_otyper = read_volatile(GPIOB_OTYPER);
        gpiob_otyper &= !(1 << DRIVE_WL_PIN);
        write_volatile(GPIOB_OTYPER, gpiob_otyper);

        let mut gpiob_ospeedr = read_volatile(GPIOB_OSPEEDR);
        gpiob_ospeedr &= !(0b11 << (DRIVE_WL_PIN * 2));
        write_volatile(GPIOB_OSPEEDR, gpiob_ospeedr);

        let mut gpiob_pupdr = read_volatile(GPIOB_PUPDR);
        gpiob_pupdr &= !(0b11 << (OP3_OUT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (POT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (TEMP_PIN * 2));
        gpiob_pupdr &= !(0b11 << (DRIVE_WL_PIN * 2));
        write_volatile(GPIOB_PUPDR, gpiob_pupdr);

        let mut gpiob_ascr = read_volatile(GPIOB_ASCR);
        gpiob_ascr |= 1 << OP3_OUT_PIN;
        gpiob_ascr |= 1 << POT_PIN;
        gpiob_ascr |= 1 << TEMP_PIN;
        write_volatile(GPIOB_ASCR, gpiob_ascr);

        // GPIOC:
        // PC6  = STATUS LED output
        // PC10 = button input
        // PC13 = UL GPIO output LOW
        let mut gpioc_moder = read_volatile(GPIOC_MODER);

        gpioc_moder &= !(0b11 << (STATUS_LED_PIN * 2));
        gpioc_moder |= 0b01 << (STATUS_LED_PIN * 2);

        gpioc_moder &= !(0b11 << (USER_BUTTON_PIN * 2));

        gpioc_moder &= !(0b11 << (DRIVE_UL_PIN * 2));
        gpioc_moder |= 0b01 << (DRIVE_UL_PIN * 2);

        write_volatile(GPIOC_MODER, gpioc_moder);

        let mut gpioc_otyper = read_volatile(GPIOC_OTYPER);
        gpioc_otyper &= !(1 << STATUS_LED_PIN);
        gpioc_otyper &= !(1 << DRIVE_UL_PIN);
        write_volatile(GPIOC_OTYPER, gpioc_otyper);

        let mut gpioc_ospeedr = read_volatile(GPIOC_OSPEEDR);
        gpioc_ospeedr &= !(0b11 << (STATUS_LED_PIN * 2));
        gpioc_ospeedr &= !(0b11 << (DRIVE_UL_PIN * 2));
        write_volatile(GPIOC_OSPEEDR, gpioc_ospeedr);

        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (STATUS_LED_PIN * 2));
        gpioc_pupdr &= !(0b11 << (USER_BUTTON_PIN * 2));
        gpioc_pupdr |= 0b01 << (USER_BUTTON_PIN * 2);
        gpioc_pupdr &= !(0b11 << (DRIVE_UL_PIN * 2));
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);

        // Force drive pins LOW again after output mode is active.
        force_drive_pins_low();
    }
}

fn setup_tim1_internal_counter_moe_off() {
    unsafe {
        // Enable TIM1 peripheral clock.
        let apb2enr = read_volatile(RCC_APB2ENR);
        write_volatile(RCC_APB2ENR, apb2enr | RCC_APB2ENR_TIM1EN);

        asm::delay(8_000);

        // Stop counter while configuring.
        write_volatile(TIM1_CR1, 0);

        // Disable all capture/compare outputs.
        write_volatile(TIM1_CCER, 0);

        // Keep MOE off. No main output enable.
        let bdtr = read_volatile(TIM1_BDTR);
        write_volatile(TIM1_BDTR, bdtr & !TIM1_BDTR_MOE);

        // Basic internal timer setup.
        write_volatile(TIM1_PSC, TIM1_TEST_PSC);
        write_volatile(TIM1_ARR, TIM1_TEST_ARR);
        write_volatile(TIM1_RCR, 0);

        // Duty registers remain zero.
        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        // Configure OC1/OC2/OC3 as PWM mode 1 internally.
        // Outputs remain disabled by CCER=0 and MOE=0, and pins remain GPIO LOW.
        let ccmr1 = (0b110 << 4) | (1 << 3) | (0b110 << 12) | (1 << 11);
        let ccmr2 = (0b110 << 4) | (1 << 3);

        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);

        // Generate update event so PSC/ARR preload takes effect.
        write_volatile(TIM1_EGR, TIM1_EGR_UG);

        // Start TIM1 counter internally with ARPE enabled.
        write_volatile(TIM1_CR1, TIM1_CR1_ARPE | TIM1_CR1_CEN);

        // Reassert safe external drive state.
        force_drive_pins_low();
    }
}

fn setup_adc12_common_clock() {
    unsafe {
        // ADC12 common clock mode:
        // CKMODE = 01, synchronous clock from HCLK.
        let mut ccr = read_volatile(ADC12_CCR);
        ccr &= !(0b11 << 16);
        ccr |= 0b01 << 16;
        write_volatile(ADC12_CCR, ccr);
    }
}

fn setup_single_adc(adc_base: usize) -> u32 {
    let mut status: u32 = 0;

    unsafe {
        // Exit deep-power-down and enable ADC voltage regulator.
        let mut cr = read_volatile(adc_cr(adc_base));
        cr &= !ADC_CR_DEEPPWD;
        cr |= ADC_CR_ADVREGEN;
        write_volatile(adc_cr(adc_base), cr);

        // ADC regulator startup delay.
        asm::delay(160_000);

        // Default ADC config:
        // single conversion, right-aligned, 12-bit.
        write_volatile(adc_cfgr(adc_base), 0);

        // Calibrate ADC.
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

        // Clear ADRDY before enabling.
        write_volatile(adc_isr(adc_base), ADC_ISR_ADRDY);

        // Enable ADC.
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

fn setup_adc1_channels() {
    // ADC1 channels:
    // VBUS     PA0  ADC1_IN1
    // OP1_OUT  PA2  ADC1_IN3
    // TEMP     PB14 ADC1_IN5
    // POT      PB12 ADC1_IN11
    // OP3_OUT  PB1  ADC1_IN12

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

fn setup_adc2_channels() {
    // ADC2 channels:
    // OP2_OUT PA6 ADC2_IN3

    adc_set_single_ended(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    adc_set_sample_time(ADC2_BASE, OP2_OUT_ADC_CHANNEL);

    adc_select_channel(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
}

fn setup_adc_for_board_monitor() -> (u32, u32) {
    setup_adc12_common_clock();

    setup_adc1_channels();
    let adc1_status = setup_single_adc(ADC1_BASE);

    setup_adc2_channels();
    let adc2_status = setup_single_adc(ADC2_BASE);

    (adc1_status, adc2_status)
}

// ------------------------------------------------------------
// Main
// ------------------------------------------------------------

#[entry]
fn main() -> ! {
    rtt_init_print!();

    setup_gpio();
    setup_tim1_internal_counter_moe_off();

    let (adc1_setup_status, adc2_setup_status) = setup_adc_for_board_monitor();

    force_drive_pins_low();
    let drive_startup = read_drive_pins();
    let tim1_startup = read_tim1();

    // Startup baselines. These are raw ADC comparison points only.
    let temp_startup_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
    let vbus_startup_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

    let op1_startup_raw = adc_read_channel_raw(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    let op2_startup_raw = adc_read_channel_raw(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    let op3_startup_raw = adc_read_channel_raw(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    rprintln!("================================================");
    rprintln!("B-G431B-ESC1 Rust bring-up");
    rprintln!("Version: v0.2.1-tim1-internal-counter-moe-off");
    rprintln!("PC6  = STATUS LED");
    rprintln!("PC10 = button input");
    rprintln!("PB12 = potentiometer / ADC1_IN11");
    rprintln!("PB14 = temperature feedback / ADC1_IN5");
    rprintln!("PA0  = VBUS feedback / ADC1_IN1");
    rprintln!("PA2  = OP1_OUT raw monitor / ADC1_IN3");
    rprintln!("PA6  = OP2_OUT raw monitor / ADC2_IN3");
    rprintln!("PB1  = OP3_OUT raw monitor / ADC1_IN12");
    rprintln!("Drive pins remain GPIO LOW, not TIM1 alternate function:");
    rprintln!("PA8=UH PA9=VH PA10=WH PA12=VL PB15=WL PC13=UL");
    rprintln!("TIM1 configured internally only:");
    rprintln!("CCER=0, BDTR.MOE=0, pins still GPIO LOW");
    rprintln!("ADC1 setup status: {}", adc1_setup_status);
    rprintln!("ADC2 setup status: {}", adc2_setup_status);
    rprintln!(
        "drive_startup: safe_low={} UH={} UL={} VH={} VL={} WH={} WL={}",
        drive_startup.safe_low,
        drive_startup.uh,
        drive_startup.ul,
        drive_startup.vh,
        drive_startup.vl,
        drive_startup.wh,
        drive_startup.wl
    );
    rprintln!(
        "tim1_startup: counting={} cnt_a={} cnt_b={} psc={} arr={} ccr1={} ccr2={} ccr3={} ccer={} moe={}",
        tim1_startup.counting,
        tim1_startup.cnt_a,
        tim1_startup.cnt_b,
        tim1_startup.psc,
        tim1_startup.arr,
        tim1_startup.ccr1,
        tim1_startup.ccr2,
        tim1_startup.ccr3,
        tim1_startup.ccer,
        tim1_startup.moe
    );
    rprintln!("temp_startup_raw: {}", temp_startup_raw);
    rprintln!("vbus_startup_raw: {}", vbus_startup_raw);
    rprintln!("op1_startup_raw: {}", op1_startup_raw);
    rprintln!("op2_startup_raw: {}", op2_startup_raw);
    rprintln!("op3_startup_raw: {}", op3_startup_raw);
    rprintln!("Output format:");
    rprintln!("button=<0/1> drive_safe=<0/1> UH=<0/1> UL=<0/1> VH=<0/1> VL=<0/1> WH=<0/1> WL=<0/1> tim1_counting=<0/1> tim1_moe=<0/1> tim1_ccer=<raw> pot_raw=<raw> temp_raw=<raw> vbus_raw=<raw> op1_raw=<raw> op2_raw=<raw> op3_raw=<raw> timeout=<0/1>");
    rprintln!("================================================");

    led_low();

    loop {
        // Keep L6387 inputs commanded low on every loop.
        force_drive_pins_low();

        let drive = read_drive_pins();
        let tim1 = read_tim1();

        let pot_raw = adc_read_channel_raw(ADC1_BASE, POT_ADC_CHANNEL);
        let temp_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
        let vbus_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

        let op1_raw = adc_read_channel_raw(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
        let op2_raw = adc_read_channel_raw(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
        let op3_raw = adc_read_channel_raw(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

        let pot_timeout = pot_raw == ADC_TIMEOUT_VALUE;
        let temp_timeout = temp_raw == ADC_TIMEOUT_VALUE;
        let vbus_timeout = vbus_raw == ADC_TIMEOUT_VALUE;
        let op1_timeout = op1_raw == ADC_TIMEOUT_VALUE;
        let op2_timeout = op2_raw == ADC_TIMEOUT_VALUE;
        let op3_timeout = op3_raw == ADC_TIMEOUT_VALUE;

        let timeout = pot_timeout
            || temp_timeout
            || vbus_timeout
            || op1_timeout
            || op2_timeout
            || op3_timeout;

        let live_pot_raw = if pot_timeout { 2048 } else { pot_raw };
        let live_temp_raw = if temp_timeout { temp_startup_raw } else { temp_raw };
        let live_vbus_raw = if vbus_timeout { vbus_startup_raw } else { vbus_raw };

        let live_op1_raw = if op1_timeout { op1_startup_raw } else { op1_raw };
        let live_op2_raw = if op2_timeout { op2_startup_raw } else { op2_raw };
        let live_op3_raw = if op3_timeout { op3_startup_raw } else { op3_raw };

        let pot_pct = adc_live_percent(live_pot_raw);
        let temp_delta = adc_delta(live_temp_raw, temp_startup_raw);
        let vbus_delta = adc_delta(live_vbus_raw, vbus_startup_raw);

        let op1_delta = adc_delta(live_op1_raw, op1_startup_raw);
        let op2_delta = adc_delta(live_op2_raw, op2_startup_raw);
        let op3_delta = adc_delta(live_op3_raw, op3_startup_raw);

        let delay = pot_to_delay(live_pot_raw);

        if button_pressed() {
            rprintln!(
                "button=1 drive_safe={} UH={} UL={} VH={} VL={} WH={} WL={} tim1_counting={} tim1_cnt_a={} tim1_cnt_b={} tim1_arr={} tim1_ccr1={} tim1_ccr2={} tim1_ccr3={} tim1_ccer={} tim1_moe={} pot_raw={} pot_pct={} temp_raw={} temp_delta={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} delay_cycles={} timeout={} mode=button_fast",
                drive.safe_low,
                drive.uh,
                drive.ul,
                drive.vh,
                drive.vl,
                drive.wh,
                drive.wl,
                tim1.counting,
                tim1.cnt_a,
                tim1.cnt_b,
                tim1.arr,
                tim1.ccr1,
                tim1.ccr2,
                tim1.ccr3,
                tim1.ccer,
                tim1.moe,
                live_pot_raw,
                pot_pct,
                live_temp_raw,
                temp_delta,
                live_vbus_raw,
                vbus_delta,
                live_op1_raw,
                op1_delta,
                live_op2_raw,
                op2_delta,
                live_op3_raw,
                op3_delta,
                650_000,
                if timeout { 1 } else { 0 }
            );

            blink_fast_button_override();
        } else {
            rprintln!(
                "button=0 drive_safe={} UH={} UL={} VH={} VL={} WH={} WL={} tim1_counting={} tim1_cnt_a={} tim1_cnt_b={} tim1_arr={} tim1_ccr1={} tim1_ccr2={} tim1_ccr3={} tim1_ccer={} tim1_moe={} pot_raw={} pot_pct={} temp_raw={} temp_delta={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} delay_cycles={} timeout={} mode=pot_control",
                drive.safe_low,
                drive.uh,
                drive.ul,
                drive.vh,
                drive.vl,
                drive.wh,
                drive.wl,
                tim1.counting,
                tim1.cnt_a,
                tim1.cnt_b,
                tim1.arr,
                tim1.ccr1,
                tim1.ccr2,
                tim1.ccr3,
                tim1.ccer,
                tim1.moe,
                live_pot_raw,
                pot_pct,
                live_temp_raw,
                temp_delta,
                live_vbus_raw,
                vbus_delta,
                live_op1_raw,
                op1_delta,
                live_op2_raw,
                op2_delta,
                live_op3_raw,
                op3_delta,
                delay,
                if timeout { 1 } else { 0 }
            );

            blink_with_delay(delay);
        }
    }
}

// ================================================================
// Footer
// File: main.rs
// Version: v0.2.1-tim1-internal-counter-moe-off
// Created: 2026-06-07
// Generated timestamp: 2026-06-07
// ================================================================