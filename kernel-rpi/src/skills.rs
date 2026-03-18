//! Skill runtime — structured dispatch for built-in commands.
//! Future: load from SD card SKILL.md when block driver exists.

use crate::{eval_simple, eq_ignore_ascii_case, uart_write, UartWriter};
use core::fmt::Write;

fn skill_help() {
    uart_write(b"Commands: help, time, clear, version, weather [loc], calc <expr>, ask <q>. (Ctrl+A X exits)");
}

fn skill_time() {
    let (ticks, freq) = aios_hal_bare::timer::read();
    let secs = if freq > 0 { ticks / freq } else { 0 };
    let _ = Write::write_fmt(
        &mut UartWriter,
        core::format_args!("Time: {} s since boot (ticks={}, freq={} Hz)", secs, ticks, freq),
    );
}

fn skill_clear() {
    uart_write(b"\x1b[2J\x1b[H");
}

fn skill_version() {
    uart_write(b"AIOS kernel-rpi 0.1.0 (aarch64 bare-metal)");
}

fn skill_weather(_rest: &str) {
    uart_write(b"Weather: 20C, clear (mock)");
}

fn skill_calc(rest: &str) {
    match eval_simple(rest) {
        Some(r) => {
            let _ = Write::write_fmt(&mut UartWriter, core::format_args!("{} = {}", rest, r));
        }
        None => uart_write(b"calc: use format N op N (e.g. 2+3)"),
    }
}

fn skill_ask(rest: &str) {
    match aios_hal_bare::inference::inference(rest) {
        Ok(r) => uart_write(r.as_bytes()),
        Err(e) => uart_write(e.as_bytes()),
    }
}

/// Dispatch line to first matching skill. Returns true if handled.
pub fn dispatch(line: &str) -> bool {
    if line.is_empty() {
        return true;
    }
    if eq_ignore_ascii_case(line, "help") {
        skill_help();
        return true;
    }
    if eq_ignore_ascii_case(line, "time") {
        skill_time();
        return true;
    }
    if eq_ignore_ascii_case(line, "clear") {
        skill_clear();
        return true;
    }
    if eq_ignore_ascii_case(line, "version") {
        skill_version();
        return true;
    }
    if line.len() >= 7 && eq_ignore_ascii_case(&line[..7], "weather") {
        skill_weather(line[7..].trim());
        return true;
    }
    if line.len() > 11 && eq_ignore_ascii_case(&line[..11], "calculator ") {
        skill_calc(line[11..].trim());
        return true;
    }
    if line.len() > 5 && eq_ignore_ascii_case(&line[..5], "calc ") {
        skill_calc(line[5..].trim());
        return true;
    }
    if line.len() > 4 && eq_ignore_ascii_case(&line[..4], "ask ") {
        skill_ask(line[4..].trim());
        return true;
    }
    false
}
