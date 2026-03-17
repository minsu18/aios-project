#!/usr/bin/env node
/**
 * AIOS Phase 1 Prototype — Entry point
 *
 * Usage:
 *   node dist/index.js          # Interactive demo
 *   node dist/index.js demo     # Run demo prompts
 *   node dist/index.js skills   # List loaded skills and tools
 */

import { process as processInput } from "./ai-core.js";
import { loadAllSkills, listTools } from "./skill-runtime.js";

function main() {
  const cmd = process.argv[2];

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

  if (cmd === "demo") {
    const prompts = [
      "What time is it?",
      "Hello!",
      "What's the weather in Tokyo?",
      "Explain quantum computing and its applications in drug discovery",
    ];
    console.log("AIOS Prototype — Demo\n");
    for (const p of prompts) {
      const result = processInput(p);
      console.log(`> ${p}`);
      console.log(`  [${result.target}] ${result.intent}: ${result.message}\n`);
    }
    return;
  }

  // Default: single prompt from args or stdin
  const input = process.argv.slice(2).join(" ").trim();
  if (input) {
    const result = processInput(input);
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log("AIOS Phase 1 Prototype");
    console.log("Usage: node dist/index.js [prompt] | demo | skills");
  }
}

main();
