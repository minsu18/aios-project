//! AIOS Kernel — Raspberry Pi 3/4 (aarch64)
//!
//! Bare-metal boot, PL011 UART I/O, rule-based conversation loop.

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::alloc::Layout;
use core::panic::PanicInfo;

mod allocator;
mod block;
mod bridge;
mod fat32;
mod pm;
mod skill_registry;
mod skills;

#[global_allocator]
static ALLOC: allocator::BumpAllocator = allocator::BumpAllocator;

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    uart_write(b"alloc error: layout too large\r\n");
    loop {
        unsafe { core::arch::asm!("wfe"); }
    }
}

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
    aios_hal_bare::init();
    uart_write(b"[ 0.001] HAL init\r\n");
    uart_write(b"[ 0.002] AI layer: host bridge\r\n");
    uart_write(b"\r\n>> AIOS kernel ready. help, time, load, skills, mem, sd, uptime, cpuinfo, reboot, weather, calc, ask\r\n>> ");

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

pub fn uart_write(s: &[u8]) {
    for &b in s {
        unsafe { uart_putc(b) }
    }
}

pub(crate) unsafe fn uart_read_byte() -> u8 {
    let base = UART_BASE as *const u32;
    while base.add(UARTFR as usize / 4).read_volatile() & UARTFR_RXFE != 0 {}
    base.add(UARTDR as usize / 4).read_volatile() as u8
}

/// Non-blocking read. Returns None if RX FIFO is empty.
pub(crate) unsafe fn uart_try_read_byte() -> Option<u8> {
    let base = UART_BASE as *const u32;
    if base.add(UARTFR as usize / 4).read_volatile() & UARTFR_RXFE != 0 {
        return None;
    }
    Some(base.add(UARTDR as usize / 4).read_volatile() as u8)
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
    // Discard stray bridge protocol lines (late reply after timeout)
    if line.starts_with("AIOS_BRIDGE_REPLY:") || line.starts_with("AIOS_BRIDGE_ASK:") {
        return;
    }
    if !skills::dispatch(line) {
        uart_write(line.as_bytes());
    }
}

/// Minimal calculator: "a+b", "a-b", "a*b", "a/b"
pub fn eval_simple(expr: &str) -> Option<i64> {
    let expr = expr.trim();
    let op_pos = expr.find(|c| c == '+' || c == '-' || c == '*' || c == '/')?;
    let left: i64 = expr[..op_pos].trim().parse().ok()?;
    let right: i64 = expr[op_pos + 1..].trim().parse().ok()?;
    let op = expr.as_bytes().get(op_pos)?;
    let r = match *op {
        b'+' => left.checked_add(right)?,
        b'-' => left.checked_sub(right)?,
        b'*' => left.checked_mul(right)?,
        b'/' => left.checked_div(right)?,
        _ => return None,
    };
    Some(r)
}

pub fn eq_ignore_ascii_case(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .all(|(x, y)| x.eq_ignore_ascii_case(&y))
}

pub struct UartWriter;

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
