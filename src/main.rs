// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.5.25-injected-ab-op2-op3v
// Purpose: STM32G431CB Rust: open-loop sine/SPWM motor test plus
//          TIM1_CH4-triggered injected ADC A/B diagnostics:
//            A = ADC1 OP1 + ADC2 OP2
//            B = ADC1 OP1 + ADC2 OP3/VOPAMP3
//          with OP3/VOPAMP3 regular ADC comparison for the
//          ST B-G431B-ESC1 board.
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.5.24:
//   - Updates startup text and runtime notes for injected-channel A/B test.
//   - Keeps motor-drive behavior unchanged.
//   - Keeps OP1/OP2 as the proven A pair.
//   - Adds B-pair diagnostic wording for ADC2 injected VOPAMP3.
//   - No phase-current reconstruction, no FOC, no current loop.
// ================================================================

#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

mod regs;
mod gpio;
mod drive;
mod tim1;
mod adc;
#[allow(dead_code)]
mod bemf;
mod safety;
mod log;
#[allow(dead_code)]
mod sixstep;
mod sine;
mod opamp;
mod current_sense;

use crate::adc::*;
use crate::current_sense::*;
use crate::drive::*;
use crate::gpio::*;
use crate::log::*;
use crate::opamp::*;
use crate::regs::*;
use crate::safety::*;
use crate::sine::*;
use crate::tim1::*;

// ------------------------------------------------------------
// Peripheral setup
// ------------------------------------------------------------

fn setup_gpio_base() {
    unsafe {
        let rcc_ahb2enr = read_volatile(RCC_AHB2ENR);
        write_volatile(
            RCC_AHB2ENR,
            rcc_ahb2enr
                | RCC_AHB2ENR_GPIOAEN
                | RCC_AHB2ENR_GPIOBEN
                | RCC_AHB2ENR_GPIOCEN
                | RCC_AHB2ENR_ADC12EN,
        );

        delay_cycles(8_000);

        force_drive_output_latches_low();

        set_pin_mode(GPIOA_MODER, VBUS_PIN, 0b11);
        set_pin_mode(GPIOA_MODER, OP1_INP_PIN, 0b11);
        set_pin_mode(GPIOA_MODER, OP1_OUT_PIN, 0b11);
        set_pin_mode(GPIOA_MODER, OP2_OUT_PIN, 0b11);
        set_pin_mode(GPIOA_MODER, OP2_INP_PIN, 0b11);

        let mut gpioa_pupdr = read_volatile(GPIOA_PUPDR);
        gpioa_pupdr &= !(0b11 << (VBUS_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP1_INP_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP1_OUT_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP2_OUT_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP2_INP_PIN * 2));
        write_volatile(GPIOA_PUPDR, gpioa_pupdr);

        let mut gpioa_ascr = read_volatile(GPIOA_ASCR);
        gpioa_ascr |= 1 << VBUS_PIN;
        gpioa_ascr |= 1 << OP1_INP_PIN;
        gpioa_ascr |= 1 << OP1_OUT_PIN;
        gpioa_ascr |= 1 << OP2_OUT_PIN;
        gpioa_ascr |= 1 << OP2_INP_PIN;
        write_volatile(GPIOA_ASCR, gpioa_ascr);

        set_pin_mode(GPIOB_MODER, OP3_INP_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, OP3_OUT_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, POT_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, TEMP_PIN, 0b11);

        let mut gpiob_pupdr = read_volatile(GPIOB_PUPDR);
        gpiob_pupdr &= !(0b11 << (OP3_INP_PIN * 2));
        gpiob_pupdr &= !(0b11 << (OP3_OUT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (POT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (TEMP_PIN * 2));
        write_volatile(GPIOB_PUPDR, gpiob_pupdr);

        let mut gpiob_ascr = read_volatile(GPIOB_ASCR);
        gpiob_ascr |= 1 << OP3_INP_PIN;
        gpiob_ascr |= 1 << OP3_OUT_PIN;
        gpiob_ascr |= 1 << POT_PIN;
        gpiob_ascr |= 1 << TEMP_PIN;
        write_volatile(GPIOB_ASCR, gpiob_ascr);

        set_pin_mode(GPIOC_MODER, STATUS_LED_PIN, 0b01);

        let mut gpioc_otyper = read_volatile(GPIOC_OTYPER);
        gpioc_otyper &= !(1 << STATUS_LED_PIN);
        write_volatile(GPIOC_OTYPER, gpioc_otyper);

        set_pin_mode(GPIOC_MODER, USER_BUTTON_PIN, 0b00);

        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (STATUS_LED_PIN * 2));
        gpioc_pupdr &= !(0b11 << (USER_BUTTON_PIN * 2));
        gpioc_pupdr |= 0b01 << (USER_BUTTON_PIN * 2);
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);

        led_low();
    }
}

fn setup_drive_pins_tim1_af() {
    unsafe {
        write_volatile(TIM1_CCER, 0);

        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr &= !TIM1_BDTR_MOE;
        bdtr |= TIM1_BDTR_OSSI | TIM1_BDTR_OSSR;
        write_volatile(TIM1_BDTR, bdtr);

        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        force_drive_output_latches_low();

        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_UH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_WH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VL_PIN, AF_TIM1_PA);

        set_pin_mode(GPIOA_MODER, DRIVE_UH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_VH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_WH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_VL_PIN, 0b10);

        let mut gpioa_otyper = read_volatile(GPIOA_OTYPER);
        gpioa_otyper &= !(1 << DRIVE_UH_PIN);
        gpioa_otyper &= !(1 << DRIVE_VH_PIN);
        gpioa_otyper &= !(1 << DRIVE_WH_PIN);
        gpioa_otyper &= !(1 << DRIVE_VL_PIN);
        write_volatile(GPIOA_OTYPER, gpioa_otyper);

        let mut gpioa_pupdr = read_volatile(GPIOA_PUPDR);
        gpioa_pupdr &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_VL_PIN * 2));
        write_volatile(GPIOA_PUPDR, gpioa_pupdr);

        let mut gpioa_ospeedr = read_volatile(GPIOA_OSPEEDR);
        gpioa_ospeedr &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_VL_PIN * 2));
        write_volatile(GPIOA_OSPEEDR, gpioa_ospeedr);

        set_pin_af(GPIOB_AFRL, GPIOB_AFRH, DRIVE_WL_PIN, AF_TIM1_N);
        set_pin_mode(GPIOB_MODER, DRIVE_WL_PIN, 0b10);

        let mut gpiob_otyper = read_volatile(GPIOB_OTYPER);
        gpiob_otyper &= !(1 << DRIVE_WL_PIN);
        write_volatile(GPIOB_OTYPER, gpiob_otyper);

        let mut gpiob_pupdr = read_volatile(GPIOB_PUPDR);
        gpiob_pupdr &= !(0b11 << (DRIVE_WL_PIN * 2));
        write_volatile(GPIOB_PUPDR, gpiob_pupdr);

        let mut gpiob_ospeedr = read_volatile(GPIOB_OSPEEDR);
        gpiob_ospeedr &= !(0b11 << (DRIVE_WL_PIN * 2));
        write_volatile(GPIOB_OSPEEDR, gpiob_ospeedr);

        set_pin_af(GPIOC_AFRL, GPIOC_AFRH, DRIVE_UL_PIN, AF_TIM1_N);
        set_pin_mode(GPIOC_MODER, DRIVE_UL_PIN, 0b10);

        let mut gpioc_otyper = read_volatile(GPIOC_OTYPER);
        gpioc_otyper &= !(1 << DRIVE_UL_PIN);
        write_volatile(GPIOC_OTYPER, gpioc_otyper);

        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (DRIVE_UL_PIN * 2));
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);

        let mut gpioc_ospeedr = read_volatile(GPIOC_OSPEEDR);
        gpioc_ospeedr &= !(0b11 << (DRIVE_UL_PIN * 2));
        write_volatile(GPIOC_OSPEEDR, gpioc_ospeedr);

        apply_state(DriveState::IdleAllOff);
    }
}

// ------------------------------------------------------------
// Main
// ------------------------------------------------------------

#[entry]
fn main() -> ! {
    rtt_init_print!();

    setup_gpio_base();
    setup_tim1_base();
    setup_drive_pins_tim1_af();

    apply_state(DriveState::IdleAllOff);

    let opamp_status = setup_opamps_for_current_sense();
    let (adc1_setup_status, adc2_setup_status) = setup_adc_for_board_monitor();
    configure_current_sense_adc_sample_time_for_sync();
    let injected_config = configure_current_sense_injected_tim1_ch4();

    let baseline_seed = read_adc_snapshot();
    let current_offsets = capture_current_sense_offsets(CURRENT_SENSE_OFFSET_SAMPLES);
    let baseline = adc_baseline_with_current_offsets(baseline_seed, current_offsets);
    let startup_drive = read_drive();
    let startup_tim1 = read_tim1_for_state(DriveState::IdleAllOff);

    let startup_pins_ok = pins_match_state(startup_drive, DriveState::IdleAllOff);

    let startup_ok = if startup_drive.af_ok == 1
        && startup_pins_ok == 1
        && no_phase_overlap(startup_drive) == 1
        && startup_tim1.tim1_basic_ok == 1
        && baseline.timeout == 0
        && opamp_status.setup_ok == 1
        && current_offsets.timeout == 0
        && current_offsets.op3_vopamp3_timeout == 0
    {
        1
    } else {
        0
    };

    rprintln!("================================================");
    rprintln!("B-G431B-ESC1 Rust bring-up");
    rprintln!("Version: v0.5.25-injected-ab-op2-op3v");
    rprintln!("Mode: open-loop sine/SPWM test");
    rprintln!("Button released: all outputs off.");
    rprintln!("Button held: precharge -> sine align -> open-loop sine ramp.");
    rprintln!("Release button to stop immediately.");
    rprintln!("No prop. Low voltage. Strict bench current limit.");
    rprintln!(
        "PWM freq = timer_clk / {} (ARR+1). On HSI16 this is ~20 kHz.",
        TIM1_TEST_ARR + 1
    );
    rprintln!("ADC1 setup status: {}", adc1_setup_status);
    rprintln!("ADC2 setup status: {}", adc2_setup_status);

    rprintln!("OPAMP current-sense bring-up:");
    rprintln!("  OPAMP1: VINP0 PA1 -> VOUT PA2 -> ADC1_IN3");
    rprintln!("  OPAMP2: VINP0 PA7 -> VOUT PA6 -> ADC2_IN3");
    rprintln!("  OPAMP3 external: VINP0 PB0 -> VOUT PB1 -> ADC1_IN12");
    rprintln!("  OPAMP3/VOPAMP3 internal route: ADC2_IN18");
    rprintln!("  mode: PGA, gain x16, high-speed, factory trim");
    rprintln!("  note: raw ADC-count logging only; not calibrated amps");

    rprintln!(
        "  injected A/B diagnostic: trigger=TIM1_CH4 target_cnt={} A=ADC1_OP1_PLUS_ADC2_OP2 B=ADC1_OP1_PLUS_ADC2_OP3V sample_bits={} max_wait_loops={}",
        CURRENT_SENSE_INJECTED_TIM1_CCR4,
        CURRENT_SENSE_SYNC_SAMPLE_BITS,
        CURRENT_SENSE_INJECTED_WAIT_MAX_LOOPS
    );

    rprintln!(
        "OPAMP status: setup_ok={} en1={} en2={} en3={} cfg1_ok={} cfg2_ok={} cfg3_ok={} op3_vopamp3_internal_ok={} csr1=0x{:08x} csr2=0x{:08x} csr3=0x{:08x}",
        opamp_status.setup_ok,
        opamp_status.en1,
        opamp_status.en2,
        opamp_status.en3,
        opamp_status.cfg1_ok,
        opamp_status.cfg2_ok,
        opamp_status.cfg3_ok,
        opamp_status.op3_vopamp3_internal_ok,
        opamp_status.csr1,
        opamp_status.csr2,
        opamp_status.csr3
    );

    rprintln!(
        "Current-sense zero offsets: timeout={} samples_used={} samples_requested={} op1_zero={} op2_zero={} op3_zero={} op3v_zero={} op3v_samples_used={} op3v_timeout={}",
        current_offsets.timeout,
        current_offsets.samples_used,
        current_offsets.samples_requested,
        current_offsets.op1_zero,
        current_offsets.op2_zero,
        current_offsets.op3_zero,
        current_offsets.op3_vopamp3_zero,
        current_offsets.op3_vopamp3_samples_used,
        current_offsets.op3_vopamp3_timeout
    );

    rprintln!(
        "FOC-prep injected setup: setup_ok={} target_cnt={} tim1_ccr4={} ch4_oc_ok={} tim1_ccmr2=0x{:08x} tim1_ccer=0x{:08x} adc1_jsqr=0x{:08x} adc2_jsqr=0x{:08x} adc2_channel={} adc1_cfgr=0x{:08x} adc2_cfgr=0x{:08x} boot_pair=ADC1_OP1_ADC2_OP2 runtime_ab=A_OP2_B_OP3V trigger=TIM1_CH4",
        injected_config.setup_ok,
        injected_config.target_cnt,
        injected_config.tim1_ccr4,
        injected_config.tim1_ch4_oc_ok,
        injected_config.tim1_ccmr2,
        injected_config.tim1_ccer,
        injected_config.adc1_jsqr,
        injected_config.adc2_jsqr,
        injected_config.adc2_channel,
        injected_config.adc1_cfgr,
        injected_config.adc2_cfgr
    );

    rprintln!("Sine/SPWM:");
    rprintln!("  TIM1 complementary outputs: CH1/CH1N CH2/CH2N CH3/CH3N");
    rprintln!("  center CCR:      {}", SINE_PWM_CENTER);
    rprintln!("  min CCR clamp:   {}", SINE_PWM_MIN_DUTY);
    rprintln!("  max CCR clamp:   {}", SINE_PWM_MAX_DUTY);
    rprintln!("  start amplitude: {}", SINE_PWM_START_AMPLITUDE);
    rprintln!("  max amplitude:   {}", SINE_PWM_RUN_MAX_AMPLITUDE);
    rprintln!("  amp inc / erev:  {}", SINE_PWM_INC_PER_ELECTRICAL_REV);
    rprintln!("  table length:    {}", SINE_TABLE_LEN);
    rprintln!("  deadtime DTG:    {}", TIM1_BDTR_SINE_SAFE_DTG);

    rprintln!("Important:");
    rprintln!("  BEMF observe is not used in sine/SPWM mode.");
    rprintln!("  Potentiometer is read only; it does not control this test.");
    rprintln!("  This is open-loop angle generation only.");
    rprintln!("  focmap_ab lines run two injected conversions:");
    rprintln!("    A: ADC1 injected OP1, ADC2 injected OP2");
    rprintln!("    B: ADC1 injected OP1, ADC2 injected OP3/VOPAMP3");
    rprintln!("  OP3/VOPAMP3 regular ADC read is also logged for comparison.");
    rprintln!("  Current-sense lines are observation-only and do not affect drive output.");
    rprintln!("  No phase-current reconstruction or FOC control has been added.");

    rprintln!(
        "startup: startup_ok={} opamp_ok={} op3v_internal_ok={} cs_offset_ok={} op3v_offset_ok={} af_ok={} pins_ok={} no_phase_overlap={} tim1_ok={} UH={} UL={} VH={} VL={} WH={} WL={} ccer={} moe={} forced_modes_ok={} pot_raw={} temp_raw={} vbus_raw={} op1_raw={} op2_raw={} op3_raw={} timeout={}",
        startup_ok,
        opamp_status.setup_ok,
        opamp_status.op3_vopamp3_internal_ok,
        if current_offsets.timeout == 0 { 1 } else { 0 },
        if current_offsets.op3_vopamp3_timeout == 0 { 1 } else { 0 },
        startup_drive.af_ok,
        startup_pins_ok,
        no_phase_overlap(startup_drive),
        startup_tim1.tim1_basic_ok,
        startup_drive.uh_pin,
        startup_drive.ul_pin,
        startup_drive.vh_pin,
        startup_drive.vl_pin,
        startup_drive.wh_pin,
        startup_drive.wl_pin,
        startup_tim1.ccer,
        startup_tim1.moe,
        startup_tim1.forced_modes_ok,
        baseline.pot_raw,
        baseline.temp_raw,
        baseline.vbus_raw,
        baseline.op1_raw,
        baseline.op2_raw,
        baseline.op3_raw,
        baseline.timeout
    );

    rprintln!("Expected idle: startup_ok=1 UH=0 UL=0 VH=0 VL=0 WH=0 WL=0");
    rprintln!("Watch focmap_ab: ok_a=1 ok_b=1 a_jeos1=1 a_jeos2=1 b_jeos1=1 b_jeos2=1 a_to=0 b_to=0.");
    rprintln!("Safety stops: button release, health fault, max step count.");
    rprintln!("================================================");

    log_current_sense(0, "startup", current_offsets);

    let mut run_id: u32 = 1;

    loop {
        apply_state(DriveState::IdleAllOff);

        if button_pressed() {
            log_current_sense(run_id, "pre_run", current_offsets);
            run_sine_openloop(run_id, baseline, current_offsets);
            log_current_sense(run_id, "post_run", current_offsets);

            while button_pressed() {
                apply_state(DriveState::IdleAllOff);
                delay_cycles(RELEASE_CHECK_CHUNK);
            }

            run_id = run_id.wrapping_add(1);
        } else {
            led_high();
            delay_cycles(IDLE_LED_ON_DELAY);
            led_low();

            log_state(run_id, 0, 0, 0, DriveState::IdleAllOff, baseline);
            log_current_sense(run_id, "idle", current_offsets);

            delay_cycles(IDLE_LED_OFF_DELAY);
            delay_cycles(IDLE_LOG_DELAY);
        }
    }
}

// ================================================================
// Footer
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.5.25-injected-ab-op2-op3v
// Created: 2026-06-08
// Generated timestamp: 2026-06-08T18:00:00Z
// ================================================================