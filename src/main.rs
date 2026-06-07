// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.1.7-pb14-temperature-feedback-raw
// Purpose: STM32G431CB Rust bring-up: live PB12 pot + PB14 temperature feedback raw ADC over RTT
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
const GPIOB_BASE: usize = 0x4800_0400;
const GPIOC_BASE: usize = 0x4800_0800;
const ADC1_BASE: usize = 0x5000_0000;
const ADC12_COMMON_BASE: usize = 0x5000_0300;

// ------------------------------------------------------------
// RCC registers
// ------------------------------------------------------------

const RCC_AHB2ENR: *mut u32 = (RCC_BASE + 0x4C) as *mut u32;

const RCC_AHB2ENR_GPIOBEN: u32 = 1 << 1;
const RCC_AHB2ENR_GPIOCEN: u32 = 1 << 2;
const RCC_AHB2ENR_ADC12EN: u32 = 1 << 13;

// ------------------------------------------------------------
// GPIO registers
// ------------------------------------------------------------

const GPIOB_MODER: *mut u32 = (GPIOB_BASE + 0x00) as *mut u32;
const GPIOB_PUPDR: *mut u32 = (GPIOB_BASE + 0x0C) as *mut u32;
const GPIOB_ASCR: *mut u32 = (GPIOB_BASE + 0x2C) as *mut u32;

const GPIOC_MODER: *mut u32 = (GPIOC_BASE + 0x00) as *mut u32;
const GPIOC_PUPDR: *mut u32 = (GPIOC_BASE + 0x0C) as *mut u32;
const GPIOC_IDR: *const u32 = (GPIOC_BASE + 0x10) as *const u32;
const GPIOC_BSRR: *mut u32 = (GPIOC_BASE + 0x18) as *mut u32;

// ------------------------------------------------------------
// ADC1 registers
// ------------------------------------------------------------

const ADC1_ISR: *mut u32 = (ADC1_BASE + 0x00) as *mut u32;
const ADC1_CR: *mut u32 = (ADC1_BASE + 0x08) as *mut u32;
const ADC1_CFGR: *mut u32 = (ADC1_BASE + 0x0C) as *mut u32;
const ADC1_SMPR1: *mut u32 = (ADC1_BASE + 0x14) as *mut u32;
const ADC1_SMPR2: *mut u32 = (ADC1_BASE + 0x18) as *mut u32;
const ADC1_SQR1: *mut u32 = (ADC1_BASE + 0x30) as *mut u32;
const ADC1_DR: *const u32 = (ADC1_BASE + 0x40) as *const u32;
const ADC1_DIFSEL: *mut u32 = (ADC1_BASE + 0xB0) as *mut u32;

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

const POT_PIN: u32 = 12;         // PB12
const TEMP_PIN: u32 = 14;        // PB14

const POT_ADC_CHANNEL: u32 = 11;  // ADC1_IN11
const TEMP_ADC_CHANNEL: u32 = 5;  // ADC1_IN5

const ADC_TIMEOUT_VALUE: u16 = 0xFFFF;

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

fn adc1_select_channel(channel: u32) {
    unsafe {
        // Regular sequence length = 1 conversion.
        // SQ1 = selected channel, bits [10:6].
        let sqr1 = channel << 6;
        write_volatile(ADC1_SQR1, sqr1);
    }
}

fn adc1_read_channel_raw(channel: u32) -> u16 {
    unsafe {
        adc1_select_channel(channel);

        // Clear old EOC/EOS flags.
        write_volatile(ADC1_ISR, ADC_ISR_EOC | ADC_ISR_EOS);

        // Start regular conversion.
        let cr = read_volatile(ADC1_CR);
        write_volatile(ADC1_CR, cr | ADC_CR_ADSTART);

        // Poll for conversion complete with a timeout.
        for _ in 0..1_000_000 {
            let isr = read_volatile(ADC1_ISR);

            if (isr & ADC_ISR_EOC) != 0 {
                let raw = read_volatile(ADC1_DR) & 0x0FFF;
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
    // Pot behavior observed:
    // far right -> low / near zero
    // far left  -> high / near 4095
    //
    // low raw  -> slow blink
    // high raw -> fast blink
    //
    // Uses u64 to avoid release-mode u32 overflow.

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
        // Enable GPIOB, GPIOC, and ADC12 clocks.
        let rcc_ahb2enr = read_volatile(RCC_AHB2ENR);
        write_volatile(
            RCC_AHB2ENR,
            rcc_ahb2enr | RCC_AHB2ENR_GPIOBEN | RCC_AHB2ENR_GPIOCEN | RCC_AHB2ENR_ADC12EN,
        );

        asm::delay(8_000);

        // PC6 = output for STATUS LED.
        let mut gpioc_moder = read_volatile(GPIOC_MODER);
        gpioc_moder &= !(0b11 << (STATUS_LED_PIN * 2));
        gpioc_moder |= 0b01 << (STATUS_LED_PIN * 2);

        // PC10 = input for button.
        gpioc_moder &= !(0b11 << (USER_BUTTON_PIN * 2));
        write_volatile(GPIOC_MODER, gpioc_moder);

        // PC10 internal pull-up.
        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (USER_BUTTON_PIN * 2));
        gpioc_pupdr |= 0b01 << (USER_BUTTON_PIN * 2);
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);

        // PB12 = analog mode for potentiometer.
        // PB14 = analog mode for temperature feedback.
        let mut gpiob_moder = read_volatile(GPIOB_MODER);

        gpiob_moder &= !(0b11 << (POT_PIN * 2));
        gpiob_moder |= 0b11 << (POT_PIN * 2);

        gpiob_moder &= !(0b11 << (TEMP_PIN * 2));
        gpiob_moder |= 0b11 << (TEMP_PIN * 2);

        write_volatile(GPIOB_MODER, gpiob_moder);

        // PB12/PB14 = no pull-up / no pull-down.
        let mut gpiob_pupdr = read_volatile(GPIOB_PUPDR);

        gpiob_pupdr &= !(0b11 << (POT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (TEMP_PIN * 2));

        write_volatile(GPIOB_PUPDR, gpiob_pupdr);

        // PB12/PB14 analog switch enable.
        let mut gpiob_ascr = read_volatile(GPIOB_ASCR);
        gpiob_ascr |= 1 << POT_PIN;
        gpiob_ascr |= 1 << TEMP_PIN;
        write_volatile(GPIOB_ASCR, gpiob_ascr);
    }
}

fn setup_adc1_for_board_monitor() -> u32 {
    let mut status: u32 = 0;

    unsafe {
        // ADC12 common clock mode:
        // CKMODE = 01, synchronous clock from HCLK.
        let mut ccr = read_volatile(ADC12_CCR);
        ccr &= !(0b11 << 16);
        ccr |= 0b01 << 16;
        write_volatile(ADC12_CCR, ccr);

        // Exit deep-power-down and enable ADC voltage regulator.
        let mut cr = read_volatile(ADC1_CR);
        cr &= !ADC_CR_DEEPPWD;
        cr |= ADC_CR_ADVREGEN;
        write_volatile(ADC1_CR, cr);

        // ADC regulator startup delay.
        asm::delay(160_000);

        // Single-ended mode for both channels.
        let mut difsel = read_volatile(ADC1_DIFSEL);
        difsel &= !(1 << POT_ADC_CHANNEL);
        difsel &= !(1 << TEMP_ADC_CHANNEL);
        write_volatile(ADC1_DIFSEL, difsel);

        // Default ADC config:
        // single conversion, right-aligned, 12-bit.
        write_volatile(ADC1_CFGR, 0);

        // Long sample time for PB14 temp channel ADC1_IN5.
        // SMPR1 channel 5 uses bits [17:15].
        let mut smpr1 = read_volatile(ADC1_SMPR1);
        smpr1 &= !(0b111 << (TEMP_ADC_CHANNEL * 3));
        smpr1 |= 0b111 << (TEMP_ADC_CHANNEL * 3);
        write_volatile(ADC1_SMPR1, smpr1);

        // Long sample time for PB12 pot channel ADC1_IN11.
        // SMPR2 channel 11 uses bits [5:3], because channel 10 starts at bit 0.
        let mut smpr2 = read_volatile(ADC1_SMPR2);
        smpr2 &= !(0b111 << ((POT_ADC_CHANNEL - 10) * 3));
        smpr2 |= 0b111 << ((POT_ADC_CHANNEL - 10) * 3);
        write_volatile(ADC1_SMPR2, smpr2);

        // Default first selected channel.
        adc1_select_channel(POT_ADC_CHANNEL);

        // Calibrate ADC.
        let cr_before_cal = read_volatile(ADC1_CR);
        write_volatile(ADC1_CR, cr_before_cal | ADC_CR_ADCAL);

        let mut cal_done = false;
        for _ in 0..1_000_000 {
            if (read_volatile(ADC1_CR) & ADC_CR_ADCAL) == 0 {
                cal_done = true;
                break;
            }
        }

        if !cal_done {
            status |= 1 << 0;
        }

        // Clear ADRDY before enabling.
        write_volatile(ADC1_ISR, ADC_ISR_ADRDY);

        // Enable ADC.
        let cr_before_enable = read_volatile(ADC1_CR);
        write_volatile(ADC1_CR, cr_before_enable | ADC_CR_ADEN);

        let mut ready = false;
        for _ in 0..1_000_000 {
            if (read_volatile(ADC1_ISR) & ADC_ISR_ADRDY) != 0 {
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

// ------------------------------------------------------------
// Main
// ------------------------------------------------------------

#[entry]
fn main() -> ! {
    rtt_init_print!();

    setup_gpio();
    let adc_setup_status = setup_adc1_for_board_monitor();

    // Take a startup baseline for the raw temperature feedback.
    // This is not Celsius. It is only a raw ADC comparison point.
    let temp_startup_raw = adc1_read_channel_raw(TEMP_ADC_CHANNEL);

    rprintln!("================================================");
    rprintln!("B-G431B-ESC1 Rust bring-up");
    rprintln!("Version: v0.1.7-pb14-temperature-feedback-raw");
    rprintln!("PC6  = STATUS LED");
    rprintln!("PC10 = button input");
    rprintln!("PB12 = potentiometer / ADC1_IN11");
    rprintln!("PB14 = temperature feedback / ADC1_IN5");
    rprintln!("ADC setup status: {}", adc_setup_status);
    rprintln!("temp_startup_raw: {}", temp_startup_raw);
    rprintln!("Output format:");
    rprintln!("button=<0/1> pot_raw=<0..4095> pot_pct=<0..100> temp_raw=<0..4095> temp_delta=<raw-startup> delay_cycles=<value> timeout=<0/1>");
    rprintln!("================================================");

    led_low();

    loop {
        let pot_raw = adc1_read_channel_raw(POT_ADC_CHANNEL);
        let temp_raw = adc1_read_channel_raw(TEMP_ADC_CHANNEL);

        let pot_timeout = pot_raw == ADC_TIMEOUT_VALUE;
        let temp_timeout = temp_raw == ADC_TIMEOUT_VALUE;
        let timeout = pot_timeout || temp_timeout;

        let live_pot_raw = if pot_timeout { 2048 } else { pot_raw };
        let live_temp_raw = if temp_timeout { temp_startup_raw } else { temp_raw };

        let pot_pct = adc_live_percent(live_pot_raw);
        let temp_delta = adc_delta(live_temp_raw, temp_startup_raw);
        let delay = pot_to_delay(live_pot_raw);

        if button_pressed() {
            rprintln!(
                "button=1 pot_raw={} pot_pct={} temp_raw={} temp_delta={} delay_cycles={} timeout={} mode=button_fast",
                live_pot_raw,
                pot_pct,
                live_temp_raw,
                temp_delta,
                650_000,
                if timeout { 1 } else { 0 }
            );

            blink_fast_button_override();
        } else {
            rprintln!(
                "button=0 pot_raw={} pot_pct={} temp_raw={} temp_delta={} delay_cycles={} timeout={} mode=pot_control",
                live_pot_raw,
                pot_pct,
                live_temp_raw,
                temp_delta,
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
// Version: v0.1.7-pb14-temperature-feedback-raw
// Created: 2026-06-07
// Generated timestamp: 2026-06-07
// ================================================================