//! Skill runtime — structured dispatch for built-in commands.
//! Block driver (block.rs) scaffold exists; load from SD SKILL.md when implemented.

use crate::allocator;
use crate::{eval_simple, eq_ignore_ascii_case, uart_write, UartWriter};
use core::fmt::Write;

const SKILL_BUF_SIZE: usize = 2048;

fn skill_help() {
    uart_write(b"Commands: help, time, load, skills, mem, sd, uptime, cpuinfo, reboot, weather, calc, ask");
    #[cfg(feature = "llama")]
    uart_write(b", load_model");
    uart_write(b". (Ctrl+A X exits)");
}

/// Built-in SKILL.md for QEMU/testing when SD is unavailable.
const BUILTIN_SKILL: &[u8] = b"---
name: example
description: Example skill
version: 0.1.0
tools: [
  {\"name\":\"get_time\",\"description\":\"Get current time\"},
  {\"name\":\"echo\",\"description\":\"Echo back input\"}
]
---";

fn skill_load() {
    use crate::block::SdDevice;
    use crate::fat32::{find_root_file, read_file_content, SKILL_MD_83};
    use crate::skill_registry;
    let sd = SdDevice::new();
    if !sd.is_ready() {
        uart_write(b"load: SD unavailable, using built-in SKILL.md\r\n");
        if skill_registry::parse_and_register(BUILTIN_SKILL) {
            uart_write(b"load: example skill loaded (built-in)");
        }
        return;
    }
    let mut buf = [0u8; SKILL_BUF_SIZE];
    let (cluster, size) = match find_root_file(&sd, &SKILL_MD_83) {
        Ok(Some(x)) => x,
        Ok(None) => {
            uart_write(b"load: No SKILL.md in SD root");
            return;
        }
        Err(_) => {
            uart_write(b"load: FAT32 read error");
            return;
        }
    };
    let n = match read_file_content(&sd, cluster, size, &mut buf) {
        Ok(n) => n,
        Err(_) => {
            uart_write(b"load: File read error");
            return;
        }
    };
    if skill_registry::parse_and_register(&buf[..n]) {
        let _ = core::fmt::Write::write_fmt(
            &mut UartWriter,
            core::format_args!("load: SKILL.md loaded ({} B)", n),
        );
    } else {
        uart_write(b"load: Parse error in SKILL.md frontmatter");
    }
}

fn skill_skills() {
    crate::skill_registry::format_list(&mut |b: &[u8]| uart_write(b));
}

fn skill_uptime() {
    let (ticks, freq) = aios_hal_bare::timer::read();
    let secs = if freq > 0 { ticks / freq } else { 0 };
    let _ = Write::write_fmt(
        &mut UartWriter,
        core::format_args!("Uptime: {} s", secs),
    );
}

fn skill_cpuinfo() {
    #[cfg(target_arch = "aarch64")]
    {
        let midr: u64;
        unsafe { core::arch::asm!("mrs {}, midr_el1", out(reg) midr); }
        let implementer = ((midr >> 24) & 0xFF) as u8;
        let partno = ((midr >> 4) & 0xFFF) as u16;
        let _ = Write::write_fmt(
            &mut UartWriter,
            core::format_args!("CPU: ARM MIDR 0x{:08X} (impl 0x{:02X} part 0x{:03X})", midr, implementer, partno),
        );
    }
    #[cfg(not(target_arch = "aarch64"))]
    uart_write(b"CPU: N/A (not aarch64)");
}

fn skill_reboot() {
    uart_write(b"Rebooting...\r\n");
    crate::pm::reboot();
}

#[cfg(feature = "llama")]
fn skill_load_model() {
    use crate::block::SdDevice;
    use crate::fat32::{find_root_file, read_file_content, MODEL_GGUF_83};
    use alloc::vec::Vec;

    const MAX_MODEL_SIZE: usize = 24 * 1024 * 1024; // 24MB for small quantized models

    let sd = SdDevice::new();
    if !sd.is_ready() {
        uart_write(b"load_model: SD unavailable");
        return;
    }

    let (cluster, size) = match find_root_file(&sd, &MODEL_GGUF_83) {
        Ok(Some(x)) => x,
        Ok(None) => {
            uart_write(b"load_model: No MODEL.GGUF in SD root");
            return;
        }
        Err(_) => {
            uart_write(b"load_model: FAT32 read error");
            return;
        }
    };

    let size_usize = size as usize;
    if size_usize > MAX_MODEL_SIZE {
        let _ = core::fmt::Write::write_fmt(
            &mut UartWriter,
            core::format_args!(
                "load_model: model too large ({} MB > {} MB)",
                size_usize / (1024 * 1024),
                MAX_MODEL_SIZE / (1024 * 1024)
            ),
        );
        return;
    }

    let mut buf = Vec::new();
    if buf.try_reserve_exact(size_usize).is_err() {
        uart_write(b"load_model: allocation failed");
        return;
    }
    buf.resize(size_usize, 0);

    let n = match read_file_content(&sd, cluster, size, &mut buf) {
        Ok(n) => n,
        Err(_) => {
            uart_write(b"load_model: file read error");
            return;
        }
    };

    match aios_hal_bare::inference::init_from_memory(&buf[..n]) {
        Ok(()) => {
            let _ = core::fmt::Write::write_fmt(
                &mut UartWriter,
                core::format_args!("load_model: OK ({} KB)", n / 1024),
            );
        }
        Err(()) => {
            uart_write(b"load_model: init failed (invalid GGUF or out of memory)");
        }
    }
}

fn skill_sd() {
    use crate::block::{BlockError, BlockDevice, BLOCK_SIZE, SdDevice};
    use crate::fat32::{read_file_first_block, find_root_file, SKILL_MD_83};
    uart_write(b"SD: probing...\r\n");
    let sd = SdDevice::new();
    if sd.is_ready() {
            let blks = sd.block_count();
            let mut buf = [0u8; BLOCK_SIZE];
            match sd.read_block(0, &mut buf) {
                Ok(()) => {
                    let _ = Write::write_fmt(
                        &mut UartWriter,
                        core::format_args!(
                            "SD: OK, block 0 read",
                        ),
                    );
                    if let Some(n) = blks {
                        let _ = Write::write_fmt(
                            &mut UartWriter,
                            core::format_args!(", {} blocks", n),
                        );
                    }
                    uart_write(b". ");
                    match find_root_file(&sd, &SKILL_MD_83) {
                        Ok(Some((cluster, size))) => {
                            let _ = Write::write_fmt(
                                &mut UartWriter,
                                core::format_args!("SKILL.md found ({} B)", size),
                            );
                            if let Ok(()) = read_file_first_block(&sd, cluster, &mut buf) {
                                let end = buf.iter().position(|&b| b == 0 || b == b'\n').unwrap_or(80);
                                let end = end.min(80);
                                let pre = core::str::from_utf8(&buf[..end]).unwrap_or("?");
                                let _ = Write::write_fmt(
                                    &mut UartWriter,
                                    core::format_args!(", first: \"{}\"", pre),
                                );
                            }
                        }
                        Ok(None) => uart_write(b"No SKILL.md in root"),
                        Err(_) => uart_write(b"FAT32 read err"),
                    }
                }
                Err(BlockError::NotReady) => uart_write(b"SD: init OK, read not ready"),
                Err(BlockError::Timeout) => uart_write(b"SD: init OK, read timeout"),
                Err(BlockError::Fault(e)) => {
                    let _ = Write::write_fmt(
                        &mut UartWriter,
                        core::format_args!("SD: init OK, read fault: {}", e),
                    );
                }
            }
    } else {
        uart_write(b"SD: init timeout (no card? QEMU?)");
    }
}

fn skill_mem() {
    let (used, total) = allocator::heap_stats();
    let free = total.saturating_sub(used);
    let _ = Write::write_fmt(
        &mut UartWriter,
        core::format_args!(
            "Heap: {} KB used / {} KB total ({} KB free)",
            used / 1024,
            total / 1024,
            free / 1024
        ),
    );
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
    let r = aios_hal_bare::inference::inference(rest)
        .or_else(|_| crate::bridge::ask_host(rest));
    match r {
        Ok(s) => uart_write(s.as_bytes()),
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
    if eq_ignore_ascii_case(line, "mem") || eq_ignore_ascii_case(line, "memory") {
        skill_mem();
        return true;
    }
    if eq_ignore_ascii_case(line, "load") {
        skill_load();
        return true;
    }
    #[cfg(feature = "llama")]
    if eq_ignore_ascii_case(line, "load_model") || eq_ignore_ascii_case(line, "llama_load") {
        skill_load_model();
        return true;
    }
    if eq_ignore_ascii_case(line, "skills") {
        skill_skills();
        return true;
    }
    if eq_ignore_ascii_case(line, "sd") {
        skill_sd();
        return true;
    }
    if eq_ignore_ascii_case(line, "uptime") {
        skill_uptime();
        return true;
    }
    if eq_ignore_ascii_case(line, "cpuinfo") || eq_ignore_ascii_case(line, "cpu") {
        skill_cpuinfo();
        return true;
    }
    if eq_ignore_ascii_case(line, "reboot") {
        skill_reboot();
        return true;
    }
    if line.len() >= 7 {
        if let Some(prefix) = line.get(..7) {
            if eq_ignore_ascii_case(prefix, "weather") {
                skill_weather(line.get(7..).unwrap_or("").trim());
                return true;
            }
        }
    }
    if line.len() > 11 {
        if let Some(prefix) = line.get(..11) {
            if eq_ignore_ascii_case(prefix, "calculator ") {
                skill_calc(line.get(11..).unwrap_or("").trim());
                return true;
            }
        }
    }
    if line.len() > 5 {
        if let Some(prefix) = line.get(..5) {
            if eq_ignore_ascii_case(prefix, "calc ") {
                skill_calc(line.get(5..).unwrap_or("").trim());
                return true;
            }
        }
    }
    if line.len() > 4 {
        if let Some(prefix) = line.get(..4) {
            if eq_ignore_ascii_case(prefix, "ask ") {
                skill_ask(line.get(4..).unwrap_or("").trim());
                return true;
            }
        }
    }

    // skill.tool or skill tool args (from loaded SKILL.md)
    if let Some(dot) = line.find('.') {
        let (skill, rest) = line.split_at(dot);
        let tool_rest = rest[1..].trim();
        let (tool, args) = if let Some(sp) = tool_rest.find(|c: char| c.is_ascii_whitespace()) {
            (tool_rest[..sp].trim(), tool_rest[sp..].trim())
        } else {
            (tool_rest, "")
        };
        if crate::skill_registry::find_tool(skill.trim(), tool).is_some() {
            dispatch_tool(skill.trim(), tool, args);
            return true;
        }
    }
    let mut it = line.split_whitespace();
    let skill = match it.next() {
        Some(s) => s,
        None => return false,
    };
    let tool = match it.next() {
        Some(t) => t,
        None => return false,
    };
    let args = {
        let idx = line.find(tool).unwrap_or(0) + tool.len();
        line[idx..].trim()
    };
    if crate::skill_registry::find_tool(skill, tool).is_some() {
        dispatch_tool(skill, tool, args);
        return true;
    }
    false
}

fn dispatch_tool(skill: &str, tool: &str, args: &str) {
    if eq_ignore_ascii_case(tool, "get_time") {
        skill_time();
    } else if eq_ignore_ascii_case(tool, "echo") {
        if args.is_empty() {
            uart_write(b"echo: needs text");
        } else {
            uart_write(args.as_bytes());
        }
    } else {
        let _ = Write::write_fmt(
            &mut UartWriter,
            core::format_args!("Tool {}.{} not implemented on device", skill, tool),
        );
    }
}
