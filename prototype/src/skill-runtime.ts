/**
 * AIOS Skill Runtime prototype
 *
 * - Loads skills from ~/.aios/skills/ or .aios/skills/
 * - Parses SKILL.md (YAML frontmatter + markdown)
 * - Exposes MCP-compatible tool list
 * - Invokes tools via built-in handlers or skill's handlers.js
 */

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { homedir } from "node:os";
import { pathToFileURL } from "node:url";
import type { Skill, SkillMeta, MCPTool, ToolHandler } from "./types.js";

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

/** Built-in tool handlers (used when skill has no handlers.js) */
const BUILTIN_HANDLERS: Record<string, ToolHandler> = {
  "example.get_time": (args) => {
    const tz = (args.timezone as string) || Intl.DateTimeFormat().resolvedOptions().timeZone;
    return new Date().toLocaleTimeString(undefined, { timeZone: tz });
  },
  "example.echo": (args) => {
    return (args.text as string) ?? "";
  },
};

/** Find skill and tool by full name (e.g. "example.get_time") */
function resolveTool(skills: Skill[], fullName: string): { skill: Skill; tool: MCPTool } | null {
  for (const skill of skills) {
    for (const tool of skill.meta.tools ?? []) {
      if (`${skill.meta.name}.${tool.name}` === fullName) {
        return { skill, tool };
      }
    }
  }
  return null;
}

/**
 * Invoke an MCP tool by full name.
 * Tries: built-in handlers -> skill's handlers.js
 */
export async function invokeTool(
  skills: Skill[],
  fullToolName: string,
  args: Record<string, unknown> = {}
): Promise<{ content: unknown[]; error?: string }> {
  const resolved = resolveTool(skills, fullToolName);
  if (!resolved) {
    return { content: [], error: `Unknown tool: ${fullToolName}` };
  }

  const { skill, tool } = resolved;
  const skillDir = dirname(skill.path);

  // 1. Try built-in handlers
  const builtin = BUILTIN_HANDLERS[fullToolName];
  if (builtin) {
    try {
      const result = await Promise.resolve(builtin(args));
      return { content: [{ type: "text", text: String(result) }] };
    } catch (err) {
      return { content: [], error: String(err) };
    }
  }

  // 2. Try skill's handlers.js
  const handlersPath = join(skillDir, "handlers.js");
  if (existsSync(handlersPath)) {
    try {
      const mod = await import(pathToFileURL(handlersPath).href);
      const handler = mod[tool.name] ?? mod.default?.[tool.name];
      if (typeof handler !== "function") {
        return { content: [], error: `Handler not found: ${tool.name}` };
      }
      const result = await Promise.resolve(handler(args));
      return { content: [{ type: "text", text: JSON.stringify(result) }] };
    } catch (err) {
      return { content: [], error: String(err) };
    }
  }

  return { content: [], error: `No handler for ${fullToolName}` };
}
