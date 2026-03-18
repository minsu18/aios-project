//! SKILL.md frontmatter parser and static tool registry.
#![allow(static_mut_refs)]
//!
//! Parses YAML frontmatter (--- ... ---) to extract skill name and tools.
//! Registers for dispatch: skillname.toolname or skillname toolname args.

const MAX_TOOL_NAME: usize = 24;
const MAX_SKILL_NAME: usize = 16;
const MAX_TOOLS: usize = 8;

#[derive(Clone, Copy)]
pub struct ToolEntry {
    pub name: [u8; MAX_TOOL_NAME],
    pub len: u8,
}

#[derive(Clone, Copy)]
pub struct RegisteredSkill {
    pub name: [u8; MAX_SKILL_NAME],
    pub name_len: u8,
    pub tools: [ToolEntry; MAX_TOOLS],
    pub tool_count: u8,
}

static mut REGISTRY: Option<RegisteredSkill> = None;

fn trim_ascii(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(s.len());
    let end = s.len()
        - s.iter()
            .rev()
            .position(|&b| b != b' ' && b != b'\t' && b != b'\r')
            .unwrap_or(s.len());
    if start >= end {
        &[]
    } else {
        &s[start..end]
    }
}

/// Extract value after "name:" on its own line.
fn parse_name_line(src: &[u8]) -> Option<&[u8]> {
    let pat = b"name:";
    let idx = src.windows(pat.len()).position(|w| w == pat)?;
    let rest = &src[idx + pat.len()..];
    let end = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
    let val = trim_ascii(&rest[..end]);
    if val.is_empty() {
        return None;
    }
    Some(val)
}

/// Parse frontmatter and register. Writes into REGISTRY. Returns true on success.
pub fn parse_and_register(src: &[u8]) -> bool {
    let first = match src.windows(3).position(|w| w == b"---") {
        Some(i) => i,
        None => return false,
    };
    let after_first = first + 3;
    let rest = &src[after_first..];
    let second = match rest.windows(3).position(|w| w == b"---") {
        Some(i) => i,
        None => return false,
    };
    let front = &rest[..second];

    let name_slice = match parse_name_line(front) {
        Some(n) => n,
        None => return false,
    };

    let mut skill_name = [0u8; MAX_SKILL_NAME];
    let nlen = name_slice.len().min(MAX_SKILL_NAME);
    skill_name[..nlen].copy_from_slice(&name_slice[..nlen]);

    let pat = b"\"name\":\"";
    let mut tools = [ToolEntry {
        name: [0; MAX_TOOL_NAME],
        len: 0,
    }; MAX_TOOLS];
    let mut tcount = 0usize;

    let mut pos = 0;
    while tcount < MAX_TOOLS {
        let Some(idx) = front[pos..].windows(pat.len()).position(|w| w == pat) else {
            break;
        };
        let start = pos + idx + pat.len();
        let end = front[start..].iter().position(|&b| b == b'"').unwrap_or(0);
        if end > 0 && end <= MAX_TOOL_NAME {
            tools[tcount].name[..end].copy_from_slice(&front[start..start + end]);
            tools[tcount].len = end as u8;
            tcount += 1;
        }
        pos = start + end;
    }

    unsafe {
        REGISTRY = Some(RegisteredSkill {
            name: skill_name,
            name_len: nlen as u8,
            tools,
            tool_count: tcount as u8,
        });
    }
    true
}

/// Check if skill.tool matches a registered tool. Returns tool name slice if match.
pub fn find_tool(skill: &str, tool: &str) -> Option<&'static [u8]> {
    let reg = unsafe { REGISTRY.as_ref() }?;
    if skill.len() != reg.name_len as usize {
        return None;
    }
    if !skill.bytes().zip(reg.name.iter()).all(|(a, &b)| a == b) {
        return None;
    }
    for i in 0..reg.tool_count as usize {
        let t = &reg.tools[i];
        if t.len as usize == tool.len()
            && tool.bytes().zip(t.name.iter()).all(|(a, &b)| a == b)
        {
            return Some(&t.name[..t.len as usize]);
        }
    }
    None
}

/// Get registered skill name for display.
#[allow(dead_code)]
pub fn skill_name() -> Option<&'static [u8]> {
    let reg = unsafe { REGISTRY.as_ref() }?;
    Some(&reg.name[..reg.name_len as usize])
}

/// List registered skill and tools for "skills" command.
pub fn format_list(out: &mut dyn FnMut(&[u8])) {
    let reg = unsafe { REGISTRY.as_ref() };
    if reg.is_none() {
        out(b"No skills loaded. Put SKILL.md on SD root, run 'load'.");
        return;
    }
    let r = reg.unwrap();
    out(b"Skill: ");
    out(&r.name[..r.name_len as usize]);
    out(b" | tools: ");
    for i in 0..r.tool_count as usize {
        if i > 0 {
            out(b", ");
        }
        out(&r.tools[i].name[..r.tools[i].len as usize]);
    }
}
