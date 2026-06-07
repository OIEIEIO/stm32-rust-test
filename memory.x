/* ================================================================
 * File: memory.x
 * Path: ~/stm32-rust-test/b-g431b-esc1-rust/memory.x
 * Version: v0.1.0-pc6-status-blink
 * Purpose: Linker memory map for STM32G431CB on B-G431B-ESC1
 * Target: STM32G431CB, 128 KiB FLASH, 32 KiB RAM
 * ================================================================
 */

MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 128K
  RAM   : ORIGIN = 0x20000000, LENGTH = 32K
}

/* ================================================================
 * Footer
 * File: memory.x
 * Version: v0.1.0-pc6-status-blink
 * Created: 2026-06-07
 * Generated timestamp: 2026-06-07
 * ================================================================
 */