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
import { installSkill, removeSkill } from "./app-store.js";

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

  // Default: single prompt from args or stdin
  const input = process.argv.slice(2).join(" ").trim();
  if (input) {
    const result = await processInput(input, skills);
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log("AIOS Phase 1 Prototype");
    console.log("Usage: node dist/index.js [prompt] | demo | skills");
  }
}

main();
