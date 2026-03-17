/**
 * AIOS Skill Runtime prototype
 *
 * - Loads skills from ~/.aios/skills/ or .aios/skills/
 * - Parses SKILL.md (YAML frontmatter + markdown)
 * - Exposes MCP-compatible tool list
 */

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import type { Skill, SkillMeta, MCPTool } from "./types.js";

const SKILL_FILENAME = "SKILL.md";

/** Parse YAML-like frontmatter (minimal, no full YAML parser) */
function parseFrontmatter(content: string): { meta: Record<string, unknown>; body: string } {
  const match = content.match(/^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$/);
  if (!match) return { meta: {}, body: content };

  const [, yaml, body] = match;
  const meta: Record<string, unknown> = {};
  const lines = yaml.split("\n");
  let i = 0;

  while (i < lines.length) {
    const m = lines[i].match(/^(\w+):\s*(.*)$/);
    if (!m) {
      i++;
      continue;
    }
    let value: string = m[2].trim();
    const key = m[1];

    // Multiline JSON: consume lines until brackets balance
    if ((value.startsWith("[") || value.startsWith("{")) && !isBalanced(value)) {
      i++;
      while (i < lines.length && !isBalanced(value)) {
        value += "\n" + lines[i].trim();
        i++;
      }
    }
    meta[key] = value;
    i++;
  }
  return { meta, body: body.trim() };
}

function isBalanced(s: string): boolean {
  let brace = 0;
  let bracket = 0;
  for (const c of s) {
    if (c === "{") brace++;
    if (c === "}") brace--;
    if (c === "[") bracket++;
    if (c === "]") bracket--;
  }
  return brace === 0 && bracket === 0;
}

/** Parse tools from frontmatter (tools can be inline or referenced) */
function parseTools(meta: Record<string, unknown>): MCPTool[] {
  const raw = meta.tools;
  if (!raw || typeof raw !== "string") return [];
  try {
    return JSON.parse(raw) as MCPTool[];
  } catch {
    return [];
  }
}

/** Load a single skill from a directory */
export function loadSkill(skillDir: string): Skill | null {
  const path = join(skillDir, SKILL_FILENAME);
  if (!existsSync(path)) return null;

  const content = readFileSync(path, "utf-8");
  const { meta, body } = parseFrontmatter(content);

  const skillMeta: SkillMeta = {
    name: (meta.name as string) ?? "unknown",
    description: (meta.description as string) ?? "",
    version: (meta.version as string) ?? "0.0.0",
    tools: parseTools(meta),
  };

  return { meta: skillMeta, body, path };
}

/** Resolve skill search paths */
function getSkillPaths(): string[] {
  const paths: string[] = [];
  const cwd = process.cwd();
  const home = homedir();

  paths.push(join(cwd, ".aios", "skills"));
  paths.push(join(cwd, "..", ".aios", "skills")); // Project root when running from prototype/
  paths.push(join(home, ".aios", "skills"));

  return paths;
}

/** Load all skills from standard install paths */
export function loadAllSkills(): Skill[] {
  const skills: Skill[] = [];
  const seen = new Set<string>();

  for (const base of getSkillPaths()) {
    if (!existsSync(base)) continue;
    const entries = readdirSync(base, { withFileTypes: true });
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;
      const skillDir = join(base, entry.name);
      const skill = loadSkill(skillDir);
      if (skill && !seen.has(skill.meta.name)) {
        seen.add(skill.meta.name);
        skills.push(skill);
      }
    }
  }

  return skills;
}

/** Return MCP-compatible tools/list response (subset) */
export function listTools(skills: Skill[]): MCPTool[] {
  const tools: MCPTool[] = [];
  for (const skill of skills) {
    for (const t of skill.meta.tools ?? []) {
      tools.push({ ...t, name: `${skill.meta.name}.${t.name}` });
    }
  }
  return tools;
}
