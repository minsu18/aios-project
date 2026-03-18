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
import { c, formatResponse } from "./ui.js";

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
      console.log(c.brand("AIOS Registry"));
      console.log(c.dim(`Source: ${registryPath}\n`));
      for (const s of skills) {
        const cat = s.category ? c.dim(` [${s.category}]`) : "";
        const auth = s.author ? c.dim(` by ${s.author}`) : "";
        console.log(`  ${c.brand(s.name)} v${s.version}${cat}${auth}`);
        console.log(`    ${s.description}`);
        console.log(`    ${c.dim("source:")} ${s.source}`);
      }
    } catch (err) {
      console.error(c.error("Browse failed:"), err);
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

  if (cmd === "simulate") {
    const noVm = process.argv.includes("--no-vm");
    const cpusIdx = process.argv.indexOf("--cpus");
    const memIdx = process.argv.indexOf("--memory");
    const cpus = cpusIdx >= 0 ? process.argv[cpusIdx + 1] ?? "2" : "2";
    const mem = memIdx >= 0 ? process.argv[memIdx + 1] ?? "512" : "512";

    if (!noVm) {
      const { spawnSync } = await import("node:child_process");
      const { join } = await import("node:path");
      const script = join(process.cwd(), "..", "tools", "simulate.sh");
      const result = spawnSync("bash", [script, "--cpus", cpus, "--memory", mem], {
        stdio: "inherit",
        cwd: join(process.cwd(), ".."),
      });
      if (result.signal !== "SIGTERM" && result.status !== 0) process.exit(result.status ?? 1);
      return;
    }

    // Node-only simulated boot (no QEMU)
    console.log("\n  ___    ___   ___  ");
    console.log(" |_ _|  / _ \\ / ___|");
    console.log("  | |  | | | |\\___ \\");
    console.log("  | |  | |_| | ___) |");
    console.log(" |___|  \\___/ |____/ ");
    console.log(" AI-Native Operating System\n");
    const memNum = parseInt(mem, 10) || 512;
    const cpuNum = parseInt(cpus, 10) || 2;
    console.log("[ 0.000] Serial init");
    console.log(`[ 0.001] Memory: ${memNum} MB (simulated)`);
    console.log(`[ 0.002] CPUs: ${cpuNum} (simulated)`);
    console.log("[ 0.003] HAL init (stub)");
    console.log("[ 0.004] AI layer ready\n");
    console.log(">> AIOS prototype active. Type a prompt, 'demo', or 'exit'.\n");
    const readline = (await import("node:readline")).createInterface({
      input: process.stdin,
      output: process.stdout,
    });
    const skillsForSim = loadAllSkills();
    const prompt = () => readline.question("> ", async (line) => {
      const t = line?.trim();
      if (!t || t === "exit" || t === "quit") return readline.close();
      if (t === "demo") {
        for (const p of ["What time is it?", "Weather in Seoul?", "2+3*4", "Hello!"]) {
          const r = await processInput(p, skillsForSim);
          console.log(" ", formatResponse(r));
        }
      } else {
        const r = await processInput(t, skillsForSim);
        console.log(formatResponse(r));
      }
      prompt();
    });
    prompt();
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
    console.log(c.brand("AIOS Skills"));
    console.log(c.dim(`Loaded: ${skills.length} skills, ${tools.length} tools\n`));
    for (const s of skills) {
      const cat = s.meta.category ? c.dim(` [${s.meta.category}]`) : "";
      const auth = s.meta.author ? c.dim(` by ${s.meta.author}`) : "";
      console.log(`  ${c.brand(s.meta.name)} v${s.meta.version}${cat}${auth}`);
      console.log(`    ${s.meta.description}`);
    }
    console.log(c.dim("\nTools:"));
    for (const t of tools) {
      console.log(`  ${c.tool(t.name)} — ${t.description}`);
    }
    return;
  }

  const skills = loadAllSkills();

  if (cmd === "demo") {
    const prompts = [
      "What time is it?",
      "echo Hello from skill!",
      "Weather in Tokyo?",
      "Calculate 2+3*4",
      "Hello!",
      "Explain quantum computing briefly",
    ];
    console.log(c.brand("AIOS Demo\n"));
    for (const p of prompts) {
      const result = await processInput(p, skills);
      console.log(c.dim(">"), p);
      console.log(" ", formatResponse(result), "\n");
    }
    return;
  }

  if (cmd === "voice") {
    const { processMultimodal } = await import("./ai-core.js");
    const { captureFromMic, isDriverBridgeAvailable } = await import("./driver-bridge.js");
    try {
      let audio: Buffer;
      const useCapture = arg === "capture";
      if (arg && !useCapture) {
        const { readFileSync } = await import("node:fs");
        audio = readFileSync(arg);
      } else if (isDriverBridgeAvailable()) {
        const captured = captureFromMic(48000);
        if (!captured) {
          console.error("Voice capture failed (driver-bridge). Use voice <file> or run on Linux with ALSA.");
          process.exit(1);
        }
        audio = captured;
      } else {
        console.error("Usage: voice <file> | voice capture (requires Linux + aios-driver-bridge)");
        process.exit(1);
      }
      const result = await processMultimodal({ modality: "voice", voice: audio }, skills);
      console.log(JSON.stringify(result, null, 2));
    } catch (err) {
      console.error("Voice processing failed:", err);
      process.exit(1);
    }
    return;
  }

  if (cmd === "image") {
    const { processMultimodal } = await import("./ai-core.js");
    const { captureFromCamera, isDriverBridgeAvailable } = await import("./driver-bridge.js");
    const useCapture = !arg || arg === "capture";
    const prompt = process.argv.slice(4).join(" ") || "Describe this image.";
    try {
      let image: Buffer;
      if (arg && arg !== "capture") {
        const { readFileSync } = await import("node:fs");
        image = readFileSync(arg);
      } else if (isDriverBridgeAvailable()) {
        const captured = captureFromCamera();
        if (!captured) {
          console.error("Image capture failed (driver-bridge). Use image <file> or run on Linux with V4L2.");
          process.exit(1);
        }
        image = captured;
      } else {
        console.error("Usage: image <file> [prompt] | image capture [prompt] (requires Linux + aios-driver-bridge)");
        process.exit(1);
      }
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

  if (cmd === "interactive") {
    const readline = (await import("node:readline")).createInterface({
      input: process.stdin,
      output: process.stdout,
    });
    const prompt = () => readline.question("> ", async (line) => {
      const t = line?.trim();
      if (!t || t === "exit" || t === "quit") return readline.close();
      if (t === "demo") {
        for (const p of ["What time is it?", "Weather in Seoul?", "2+3*4", "Hello!"]) {
          const r = await processInput(p, skills);
          console.log(" ", formatResponse(r));
        }
      } else {
        const r = await processInput(t, skills);
        console.log(formatResponse(r));
      }
      prompt();
    });
    console.log(c.brand("AIOS") + " interactive. Type a prompt, 'demo', or 'exit'.");
    prompt();
    return;
  }

  // Default: single prompt from args or stdin
  const input = process.argv.slice(2).join(" ").trim();
  if (input) {
    const result = await processInput(input, skills);
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log("AIOS Phase 1 Prototype");
    console.log("Usage: [prompt] | demo | skills | simulate | interactive | voice <file>|capture | image <file>|capture [prompt] | install <path> | remove <name> | browse | install-from-registry <name> | update [name]");
  }
}

main();
