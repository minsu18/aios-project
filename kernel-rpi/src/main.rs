//! AIOS Kernel — Raspberry Pi 3/4 (aarch64)
//!
//! Bare-metal boot, PL011 UART I/O, rule-based conversation loop.

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
const UARTFR_RXFE: u32 = 1 << 4;  /* Receive FIFO empty */
const UARTCR_UARTEN: u32 = 1;
const UARTCR_TXE: u32 = 1 << 8;
const UARTCR_RXE: u32 = 1 << 9;   /* Receive enable */

#[no_mangle]
pub unsafe extern "C" fn kernel_main() -> ! {
    uart_init();
    uart_write(b"\r\n");
    uart_write(b"    ___    ________  _____\r\n");
    uart_write(b"   /   |  /  _/ __ \\/ ___/\r\n");
    uart_write(b"  / /| |  / // / / /\\__ \\ \r\n");
    uart_write(b" / ___ |_/ // /_/ /___/ / \r\n");
    uart_write(b"/_/  |_/___/\\____//____/  \r\n");
    uart_write(b"\r\n");
    uart_write(b"\r\n[ 0.000] UART init\r\n");
    uart_write(b"[ 0.001] HAL init (stub)\r\n");
    uart_write(b"[ 0.002] AI layer: host bridge\r\n");
    uart_write(b"\r\n>> AIOS kernel ready. Commands: help, time, or type to echo\r\n>> ");

    conversation_loop();
}

unsafe fn uart_init() {
    let base = UART_BASE as *mut u32;
    base.add(UARTCR as usize / 4).write_volatile(0);
    base.add(UARTIBRD as usize / 4).write_volatile(26);  /* 115200 @ 48M */
    base.add(UARTFBRD as usize / 4).write_volatile(1);
    base.add(UARTLCR_H as usize / 4).write_volatile(0x60);  /* 8N1 */
    base.add(UARTCR as usize / 4).write_volatile(UARTCR_UARTEN | UARTCR_TXE | UARTCR_RXE);
}

unsafe fn uart_putc(b: u8) {
    let base = UART_BASE as *mut u32;
    while base.add(UARTFR as usize / 4).read_volatile() & UARTFR_TXFF != 0 {}
    base.add(UARTDR as usize / 4).write_volatile(b as u32);
}

fn uart_write(s: &[u8]) {
    for &b in s {
        unsafe { uart_putc(b) }
    }
}

unsafe fn uart_read_byte() -> u8 {
    let base = UART_BASE as *const u32;
    while base.add(UARTFR as usize / 4).read_volatile() & UARTFR_RXFE != 0 {}
    base.add(UARTDR as usize / 4).read_volatile() as u8
}

const LINE_BUF: usize = 128;

fn conversation_loop() -> ! {
    let mut buf = [0u8; LINE_BUF];
    let mut len = 0usize;

    loop {
        let b = unsafe { uart_read_byte() };

        if b == b'\r' || b == b'\n' {
            uart_write(b"\r\n");
            if len > 0 {
                let line = core::str::from_utf8(&buf[..len]).unwrap_or("");
                let line = line.trim();
                handle_command(line);
                uart_write(b"\r\n");
                len = 0;
            }
            uart_write(b">> ");
        } else if b == 0x08 || b == 0x7F {
            /* Backspace or DEL: erase last char */
            if len > 0 {
                len -= 1;
                uart_write(b"\x08 \x08"); /* backspace, space, backspace */
            }
        } else if len < LINE_BUF - 1 {
            buf[len] = b;
            len += 1;
            unsafe { uart_putc(b) }
        }
    }
}

fn handle_command(line: &str) {
    if line.is_empty() {
        return;
    }
    if eq_ignore_ascii_case(line, "help") {
        uart_write(b"Commands: help, time. Or type anything to echo. (Ctrl+A X exits QEMU)");
    } else if eq_ignore_ascii_case(line, "time") {
        let (ticks, freq) = read_arm_timer();
        let secs = if freq > 0 { ticks / freq } else { 0 };
        let _ = core::fmt::Write::write_fmt(
            &mut UartWriter,
            core::format_args!("Time: {} s since boot (ticks={}, freq={} Hz)", secs, ticks, freq),
        );
    } else {
        uart_write(line.as_bytes());
    }
}

/// Read ARM Generic Timer: (CNTVCT_EL0, CNTFRQ_EL0)
fn read_arm_timer() -> (u64, u64) {
    let ticks: u64;
    let freq: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
    }
    (ticks, freq)
}

fn eq_ignore_ascii_case(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .all(|(x, y)| x.eq_ignore_ascii_case(&y))
}

struct UartWriter;

impl core::fmt::Write for UartWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        uart_write(s.as_bytes());
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    uart_write(b"PANIC: ");
    let payload = info.message();
    let _ = core::fmt::Write::write_fmt(&mut UartWriter, core::format_args!("{payload}"));
    uart_write(b"\r\n");
    loop {
        unsafe { core::arch::asm!("wfe"); }
    }
}
