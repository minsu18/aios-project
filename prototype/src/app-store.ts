/**
 * AIOS App Store — browse, install, remove skills
 *
 * Skills are installed to ~/.aios/skills/ or .aios/skills/
 */

import { cpSync, mkdirSync, readdirSync, rmSync, existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { loadAllSkills, loadSkill } from "./skill-runtime.js";
import type { Skill } from "./types.js";

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
