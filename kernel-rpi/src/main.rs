//! AIOS Kernel — Raspberry Pi 3/4 (aarch64)
//!
//! Bare-metal boot, PL011 UART output, halt.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

// boot.S is compiled by build.rs and linked separately

/// RPi 4 PL011 UART base (BCM2711) (BCM2711)
const UART_BASE: u64 = 0xFE20_1000;
const UARTDR: u64 = 0x00;  /* Data */
const UARTFR: u64 = 0x18;  /* Flags: TXFF bit 5 = FIFO full */
const UARTCR: u64 = 0x30;  /* Control */
const UARTIBRD: u64 = 0x24;
const UARTFBRD: u64 = 0x28;
const UARTLCR_H: u64 = 0x2C;

const UARTFR_TXFF: u32 = 1 << 5;  /* Transmit FIFO full */
const UARTCR_UARTEN: u32 = 1;
const UARTCR_TXE: u32 = 1 << 8;

#[no_mangle]
pub unsafe extern "C" fn kernel_main() -> ! {
    uart_init();
    uart_write(b"\r\n  ___    ___   ___  \r\n");
    uart_write(b" |_ _|  / _ \\ / ___|\r\n");
    uart_write(b"  | |  | | | |\\___ \\\r\n");
    uart_write(b"  | |  | |_| | ___) |\r\n");
    uart_write(b" |___|  \\___/ |____/ \r\n");
    uart_write(b" AIOS — Raspberry Pi\r\n");
    uart_write(b"\r\n[ 0.000] UART init\r\n");
    uart_write(b"[ 0.001] HAL init (stub)\r\n");
    uart_write(b"[ 0.002] AI layer: host bridge\r\n");
    uart_write(b"\r\n>> AIOS kernel ready.\r\n\r\n");

    loop {
        core::arch::asm!("wfe");
    }
}

unsafe fn uart_init() {
    let base = UART_BASE as *mut u32;
    base.add(UARTCR as usize / 4).write_volatile(0);
    base.add(UARTIBRD as usize / 4).write_volatile(26);  /* 115200 @ 48M */
    base.add(UARTFBRD as usize / 4).write_volatile(1);
    base.add(UARTLCR_H as usize / 4).write_volatile(0x60);  /* 8N1 */
    base.add(UARTCR as usize / 4).write_volatile(UARTCR_UARTEN | UARTCR_TXE);
}

unsafe fn uart_putc(b: u8) {
    let base = UART_BASE as *const u32;
    while base.add(UARTFR as usize / 4).read_volatile() & UARTFR_TXFF != 0 {}
    base.add(UARTDR as usize / 4).write_volatile(b as u32);
}

fn uart_write(s: &[u8]) {
    for &b in s {
        unsafe { uart_putc(b) }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    uart_write(b"PANIC: ");
    if let Some(s) = info.message().and_then(|m| m.as_str()) {
        uart_write(s.as_bytes());
    }
    uart_write(b"\r\n");
    loop {
        core::arch::asm!("wfe");
    }
}
