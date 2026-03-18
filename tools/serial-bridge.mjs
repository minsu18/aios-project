#!/usr/bin/env node
/**
 * AIOS Serial Bridge — Host inference for bare-metal kernel.
 *
 * Runs QEMU with -serial pipe, intercepts "ask" requests, calls Ollama, returns
 * responses. Usage: ./tools/simulate-rpi-bridge.sh
 *
 * Protocol: kernel sends "AIOS_BRIDGE_ASK:<prompt>\n", we reply "AIOS_BRIDGE_REPLY:<text>\n"
 *
 * Requires: Node 18+, Ollama (ollama serve, ollama pull llama3.2)
 */

import readline from "readline";
import { spawn, execSync } from "child_process";
import { createReadStream, createWriteStream, existsSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

const PIPE_BASE = process.env.AIOS_SERIAL_PIPE ?? "/tmp/aios-serial";
const OLLAMA_HOST = process.env.AIOS_OLLAMA_HOST ?? "http://127.0.0.1:11434";
const OLLAMA_MODEL = process.env.AIOS_OLLAMA_MODEL ?? "llama3.2";

const PREFIX_ASK = "AIOS_BRIDGE_ASK:";
const PREFIX_REPLY = "AIOS_BRIDGE_REPLY:";

async function callOllama(prompt) {
  try {
    const res = await fetch(`${OLLAMA_HOST}/api/generate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: OLLAMA_MODEL,
        prompt: `You are a helpful AI assistant. Reply concisely.\n\nUser: ${prompt}`,
        stream: false,
        options: { num_predict: 256 },
      }),
    });
    if (!res.ok) throw new Error(`${res.status}`);
    const data = await res.json();
    return (data.response ?? "").trim() || "(No response)";
  } catch (e) {
    return `[Ollama error: ${e.message}. Run 'ollama serve' and 'ollama pull ${OLLAMA_MODEL}']`;
  }
}

async function checkOllama() {
  try {
    const res = await fetch(`${OLLAMA_HOST}/api/tags`, { method: "GET" });
    if (res.ok) return true;
  } catch (_) {}
  return false;
}

async function main() {
  const ollamaOk = await checkOllama();
  if (!ollamaOk) {
    console.error("Ollama not reachable at", OLLAMA_HOST);
    console.error("  Start: ollama serve");
    console.error("  Model: ollama pull", OLLAMA_MODEL);
    console.error("");
    console.error("Simulation will run but 'ask' will return errors until Ollama is ready.");
    console.error("");
  }

  const pipeIn = `${PIPE_BASE}.in`;
  const pipeOut = `${PIPE_BASE}.out`;

  if (existsSync(pipeIn)) execSync(`rm -f "${pipeIn}"`);
  if (existsSync(pipeOut)) execSync(`rm -f "${pipeOut}"`);
  execSync(`mkfifo "${pipeIn}" "${pipeOut}"`);

  const elf = join(ROOT, "target/aarch64-unknown-none/release/kernel");
  const kernel = `${elf}8.img`;
  const kernelPath = existsSync(kernel) ? kernel : elf;

  const qemu = spawn("qemu-system-aarch64", [
    "-M", "raspi4b", "-m", "2G", "-cpu", "cortex-a72", "-smp", "4",
    "-kernel", kernelPath, "-serial", `pipe:${PIPE_BASE}`, "-display", "none",
  ], { stdio: ["ignore", "ignore", "inherit"] });

  let shuttingDown = false;
  function shutdown() {
    if (shuttingDown) return;
    shuttingDown = true;
    if (process.stdin.isTTY) process.stdin.setRawMode(false);
    try { qemu.kill("SIGKILL"); } catch (_) {}
    try { pipeInStream.destroy(); pipeOutStream.destroy(); } catch (_) {}
    process.exit(0);
  }
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
  console.error("(Ctrl+C or type 'quit' + Enter to exit)\n");

  const pipeOutStream = createReadStream(pipeOut);
  const pipeInStream = createWriteStream(pipeIn);

  let buf = "";
  let collecting = false;
  let prompt = "";

  pipeOutStream.on("data", async (chunk) => {
    buf += chunk.toString();
    while (buf.length > 0) {
      if (collecting) {
        const idx = buf.indexOf("\n");
        if (idx >= 0) {
          prompt += buf.slice(0, idx);
          buf = buf.slice(idx + 1);
          collecting = false;
          const reply = await callOllama(prompt.trim());
          pipeInStream.write(`${PREFIX_REPLY}${reply}\n`);
          prompt = "";
        } else {
          prompt += buf;
          buf = "";
          break;
        }
      } else {
        const idx = buf.indexOf(PREFIX_ASK);
        if (idx >= 0) {
          process.stdout.write(buf.slice(0, idx));
          buf = buf.slice(idx + PREFIX_ASK.length);
          collecting = true;
          prompt = "";
        } else {
          // Output all but keep suffix that could start PREFIX_ASK (e.g. "AIOS_BRIDGE_AS")
          let keep = 0;
          for (let n = 1; n <= Math.min(buf.length, PREFIX_ASK.length); n++) {
            const suffix = buf.slice(-n);
            if (PREFIX_ASK.startsWith(suffix)) keep = n;
          }
          const outLen = buf.length - keep;
          if (outLen > 0) {
            process.stdout.write(buf.slice(0, outLen));
            buf = buf.slice(outLen);
          }
          break;
        }
      }
    }
  });

  let quitBuf = "";
  const checkQuit = (str) => {
    if (!str) return;
    quitBuf = (quitBuf + str).slice(-8);
    if (quitBuf.endsWith("quit\n") || quitBuf.endsWith("quit\r\n")) {
      shutdown();
      return true;
    }
    return false;
  };

  if (process.stdin.isTTY) {
    process.stdin.setRawMode(true);
    readline.emitKeypressEvents(process.stdin);
    process.stdin.on("keypress", (str, key) => {
      if (key?.ctrl && key?.name === "c") {
        shutdown();
        return;
      }
      if (str != null && str !== "" && !checkQuit(str)) pipeInStream.write(str);
    });
  } else {
    process.stdin.resume();
    process.stdin.setEncoding(null);
    process.stdin.on("data", (chunk) => {
      const s = chunk.toString();
      for (let i = 0; i < chunk.length; i++) {
        if (chunk[i] === 0x03) { shutdown(); return; }
      }
      if (!checkQuit(s)) pipeInStream.write(chunk);
    });
  }

  qemu.on("exit", (code) => {
    if (!shuttingDown) process.exit(code ?? 0);
  });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
