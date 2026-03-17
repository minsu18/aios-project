/**
 * AIOS App Store — browse, install, remove skills
 *
 * Skills are installed to ~/.aios/skills/ or .aios/skills/
 * Registry: AIOS_REGISTRY_URL or local registry/skills.json
 */

import { cpSync, mkdirSync, readdirSync, rmSync, existsSync, mkdtempSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { tmpdir } from "node:os";
import { execSync } from "node:child_process";
import { loadAllSkills, loadSkill } from "./skill-runtime.js";
import type { Skill } from "./types.js";

/** Registry entry for a skill */
export interface RegistrySkill {
  name: string;
  description: string;
  version: string;
  source: string; // local:path | git:url | https://...
}

/** Default install directory (user home) */
export function getInstallDir(): string {
  return join(homedir(), ".aios", "skills");
}

/** Project-level install directory */
export function getProjectInstallDir(): string {
  return join(process.cwd(), ".aios", "skills");
}

/** List installed skills with metadata */
export function listInstalled(baseDir?: string): Skill[] {
  const dir = baseDir ?? getInstallDir();
  if (!existsSync(dir)) return [];
  const skills: Skill[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const skillDir = join(dir, entry.name);
    const skill = loadSkill(skillDir);
    if (skill) skills.push(skill);
  }
  return skills;
}

/**
 * Install a skill from a local path.
 * Copies the skill directory to the install location.
 */
export function installSkill(
  sourcePath: string,
  options: { target?: "user" | "project" } = {}
): { ok: boolean; error?: string } {
  const targetDir =
    options.target === "project" ? getProjectInstallDir() : getInstallDir();
  const skill = loadSkill(sourcePath);
  if (!skill) {
    return { ok: false, error: `Invalid skill: no SKILL.md found in ${sourcePath}` };
  }
  const destPath = join(targetDir, skill.meta.name);
  try {
    mkdirSync(targetDir, { recursive: true });
    if (existsSync(destPath)) {
      rmSync(destPath, { recursive: true });
    }
    cpSync(sourcePath, destPath, { recursive: true });
    return { ok: true };
  } catch (err) {
    return { ok: false, error: String(err) };
  }
}

/**
 * Remove an installed skill by name.
 */
export function removeSkill(
  skillName: string,
  options: { target?: "user" | "project" } = {}
): { ok: boolean; error?: string } {
  const baseDir =
    options.target === "project" ? getProjectInstallDir() : getInstallDir();
  const skillPath = join(baseDir, skillName);
  if (!existsSync(skillPath)) {
    return { ok: false, error: `Skill not found: ${skillName}` };
  }
  try {
    rmSync(skillPath, { recursive: true });
    return { ok: true };
  } catch (err) {
    return { ok: false, error: String(err) };
  }
}

/** Default remote registry URL (GitHub raw) */
const DEFAULT_REGISTRY_URL =
  "https://raw.githubusercontent.com/minsu18/aios-project/main/registry/skills.json";

/** Registry source: env URL, local file, or default remote */
function getRegistryPath(): string {
  const url = process.env.AIOS_REGISTRY_URL;
  if (url) return url;
  const projectRoot = join(process.cwd(), "..");
  const localPath = join(projectRoot, "registry", "skills.json");
  return existsSync(localPath) ? localPath : DEFAULT_REGISTRY_URL;
}

/**
 * Fetch registry and return list of available skills.
 */
export async function browseRegistry(): Promise<RegistrySkill[]> {
  const pathOrUrl = getRegistryPath();
  let json: string;

  if (pathOrUrl.startsWith("http://") || pathOrUrl.startsWith("https://")) {
    const res = await fetch(pathOrUrl);
    if (!res.ok) throw new Error(`Registry fetch failed: ${res.status}`);
    json = await res.text();
  } else {
    const { readFileSync } = await import("node:fs");
    if (!existsSync(pathOrUrl)) return [];
    json = readFileSync(pathOrUrl, "utf-8");
  }

  const data = JSON.parse(json);
  return Array.isArray(data) ? data : [];
}

/**
 * Install a skill from the registry by name.
 */
export async function installFromRegistry(
  skillName: string,
  options: { target?: "user" | "project" } = {}
): Promise<{ ok: boolean; error?: string }> {
  const skills = await browseRegistry();
  const skill = skills.find((s) => s.name === skillName);
  if (!skill) {
    return { ok: false, error: `Skill not found in registry: ${skillName}` };
  }

  const { source } = skill;
  let srcPath: string;

  if (source.startsWith("local:")) {
    const relPath = source.slice(6);
    srcPath = join(process.cwd(), "..", relPath);
    if (!existsSync(srcPath)) {
      return { ok: false, error: `Local path not found: ${srcPath}` };
    }
  } else if (source.startsWith("git:")) {
    const gitUrl = source.slice(4);
    const tmp = mkdtempSync(join(tmpdir(), "aios-skill-"));
    try {
      execSync(`git clone --depth 1 ${gitUrl} ${tmp}`, { stdio: "pipe" });
      srcPath = tmp;
    } catch (err) {
      return { ok: false, error: `Git clone failed: ${String(err)}` };
    }
  } else {
    return { ok: false, error: `Unsupported source: ${source}` };
  }

  const result = installSkill(srcPath, options);
  if (source.startsWith("git:")) {
    try {
      rmSync(srcPath, { recursive: true, force: true });
    } catch {
      /* ignore cleanup */
    }
  }
  return result;
}

/** Compare versions; returns true if a is newer than b (simple semver-like) */
function isNewerVersion(a: string, b: string): boolean {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const va = pa[i] ?? 0;
    const vb = pb[i] ?? 0;
    if (va > vb) return true;
    if (va < vb) return false;
  }
  return false;
}

/**
 * Update a skill from the registry (reinstall if newer version available).
 */
export async function updateSkill(
  skillName: string,
  options: { target?: "user" | "project" } = {}
): Promise<{ ok: boolean; updated?: boolean; error?: string }> {
  const baseDir =
    options.target === "project" ? getProjectInstallDir() : getInstallDir();
  const skillPath = join(baseDir, skillName);
  if (!existsSync(skillPath)) {
    return { ok: false, error: `Skill not installed: ${skillName}` };
  }

  const installed = loadSkill(skillPath);
  const registry = await browseRegistry();
  const reg = registry.find((s) => s.name === skillName);
  if (!reg) {
    return { ok: false, error: `Skill not in registry: ${skillName}` };
  }

  const instVersion = installed?.meta.version ?? "0.0.0";
  if (!isNewerVersion(reg.version, instVersion)) {
    return { ok: true, updated: false };
  }

  const result = await installFromRegistry(skillName, options);
  return { ok: result.ok, updated: result.ok, error: result.error };
}

/**
 * Update all installed skills that have newer versions in the registry.
 */
export async function updateAllSkills(
  options: { target?: "user" | "project" } = {}
): Promise<{ updated: string[]; errors: string[] }> {
  const installed = listInstalled(
    options.target === "project" ? getProjectInstallDir() : getInstallDir()
  );
  const registry = await browseRegistry();
  const regMap = new Map(registry.map((s) => [s.name, s]));
  const updated: string[] = [];
  const errors: string[] = [];

  for (const skill of installed) {
    const reg = regMap.get(skill.meta.name);
    if (!reg) continue;
    if (!isNewerVersion(reg.version, skill.meta.version)) continue;

    const result = await updateSkill(skill.meta.name, options);
    if (result.ok && result.updated) updated.push(skill.meta.name);
    if (result.error) errors.push(`${skill.meta.name}: ${result.error}`);
  }

  return { updated, errors };
}
