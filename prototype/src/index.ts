#!/usr/bin/env node
/**
 * AIOS Phase 1 Prototype — Entry point
 *
 * Usage:
 *   node dist/index.js [prompt]     # Single prompt
 *   node dist/index.js demo         # Run demo prompts
 *   node dist/index.js skills       # List loaded skills and tools
 *   node dist/index.js install <path>   # Install skill from path
 *   node dist/index.js remove <name>    # Remove installed skill
 */

import { process as processInput } from "./ai-core.js";
import { loadAllSkills, listTools } from "./skill-runtime.js";
import { installSkill, removeSkill, browseRegistry, installFromRegistry, updateSkill, updateAllSkills } from "./app-store.js";

async function main() {
  const cmd = process.argv[2];
  const arg = process.argv[3];

  if (cmd === "install") {
    if (!arg) {
      console.error("Usage: node dist/index.js install <path-to-skill-dir>");
      process.exit(1);
    }
    const result = installSkill(arg);
    if (result.ok) {
      console.log(`Installed skill from ${arg}`);
    } else {
      console.error(`Install failed: ${result.error}`);
      process.exit(1);
    }
    return;
  }

  if (cmd === "remove") {
    if (!arg) {
      console.error("Usage: node dist/index.js remove <skill-name>");
      process.exit(1);
    }
    const result = removeSkill(arg);
    if (result.ok) {
      console.log(`Removed skill: ${arg}`);
    } else {
      console.error(`Remove failed: ${result.error}`);
      process.exit(1);
    }
    return;
  }

  if (cmd === "browse") {
    try {
      const skills = await browseRegistry();
      const registryPath = process.env.AIOS_REGISTRY_URL ?? "default (local or GitHub)";
      console.log("Registry:", registryPath);
      console.log("Available skills:", skills.length);
      for (const s of skills) {
        console.log(`  ${s.name} v${s.version}: ${s.description}`);
        console.log(`    source: ${s.source}`);
      }
    } catch (err) {
      console.error("Browse failed:", err);
      process.exit(1);
    }
    return;
  }

  if (cmd === "install-from-registry" && arg) {
    const result = await installFromRegistry(arg);
    if (result.ok) {
      console.log(`Installed skill from registry: ${arg}`);
    } else {
      console.error(`Install failed: ${result.error}`);
      process.exit(1);
    }
    return;
  }

  if (cmd === "update") {
    if (arg) {
      const result = await updateSkill(arg);
      if (result.ok) {
        console.log(result.updated ? `Updated ${arg}` : `${arg} is already up to date`);
      } else {
        console.error(`Update failed: ${result.error}`);
        process.exit(1);
      }
    } else {
      const { updated, errors } = await updateAllSkills();
      if (updated.length) console.log("Updated:", updated.join(", "));
      if (updated.length === 0 && errors.length === 0) console.log("All skills up to date");
      for (const e of errors) console.error(e);
      if (errors.length) process.exit(1);
    }
    return;
  }

  if (cmd === "skills") {
    const skills = loadAllSkills();
    const tools = listTools(skills);
    console.log("Loaded skills:", skills.length);
    for (const s of skills) {
      console.log(`  - ${s.meta.name} v${s.meta.version}: ${s.meta.description}`);
    }
    console.log("MCP-compatible tools:", tools.length);
    for (const t of tools) {
      console.log(`  - ${t.name}: ${t.description}`);
    }
    return;
  }

  const skills = loadAllSkills();

  if (cmd === "demo") {
    const prompts = [
      "What time is it?",
      "echo Hello from skill!",
      "Hello!",
      "What's the weather in Tokyo?",
      "Explain quantum computing and its applications in drug discovery",
    ];
    console.log("AIOS Prototype — Demo\n");
    for (const p of prompts) {
      const result = await processInput(p, skills);
      const toolsStr = result.toolsUsed?.length
        ? ` [tools: ${result.toolsUsed.join(", ")}]`
        : "";
      console.log(`> ${p}`);
      console.log(`  [${result.target}] ${result.intent}: ${result.message}${toolsStr}\n`);
    }
    return;
  }

  if (cmd === "voice" && arg) {
    const { readFileSync } = await import("node:fs");
    const { processMultimodal } = await import("./ai-core.js");
    try {
      const audio = readFileSync(arg);
      const result = await processMultimodal(
        { modality: "voice", voice: audio },
        skills
      );
      console.log(JSON.stringify(result, null, 2));
    } catch (err) {
      console.error("Voice processing failed:", err);
      process.exit(1);
    }
    return;
  }

  if (cmd === "image" && arg) {
    const { readFileSync } = await import("node:fs");
    const { processMultimodal } = await import("./ai-core.js");
    const prompt = process.argv.slice(4).join(" ") || "Describe this image.";
    try {
      const image = readFileSync(arg);
      const result = await processMultimodal(
        { modality: "image", image, imagePrompt: prompt },
        skills
      );
      console.log(JSON.stringify(result, null, 2));
    } catch (err) {
      console.error("Image processing failed:", err);
      process.exit(1);
    }
    return;
  }

  // Default: single prompt from args or stdin
  const input = process.argv.slice(2).join(" ").trim();
  if (input) {
    const result = await processInput(input, skills);
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log("AIOS Phase 1 Prototype");
    console.log("Usage: node dist/index.js [prompt] | demo | skills | install <path> | remove <name> | browse | install-from-registry <name> | update [name] | voice <file> | image <file> [prompt]");
  }
}

main();
