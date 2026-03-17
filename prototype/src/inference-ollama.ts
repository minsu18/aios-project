/**
 * AIOS Ollama inference backend
 *
 * Runs local LLM via Ollama (ollama run llama3.2, etc.).
 * Set AIOS_INFERENCE=ollama. Optionally AIOS_OLLAMA_MODEL=llama3.2, AIOS_OLLAMA_HOST.
 * Requires Ollama to be running (ollama serve).
 */

import { Ollama } from "ollama";
import type { InferenceBackend } from "./inference.js";
import type { InferenceTarget } from "./types.js";
import { createPlaceholderBackend } from "./inference.js";
import { isOffline } from "./inference.js";

const ON_DEVICE_TRIGGERS = [
  "time", "date", "weather", "calculator", "calc",
  "hello", "hi", "what time", "what's the time", "echo",
];

/** Create Ollama backend. Falls back to placeholder if Ollama unavailable. */
export async function createOllamaBackend(): Promise<InferenceBackend> {
  const model = process.env.AIOS_OLLAMA_MODEL ?? "llama3.2";
  const host = process.env.AIOS_OLLAMA_HOST ?? "http://localhost:11434";
  const client = new Ollama({ host });
  const placeholder = createPlaceholderBackend();

  const available = await checkOllama(client);
  if (!available) {
    console.warn(`Ollama not reachable at ${host}. Run 'ollama serve' and 'ollama pull ${model}'. Falling back to placeholder.`);
    return placeholder;
  }

  return {
    route(input: string): InferenceTarget {
      if (isOffline()) return "on_device";
      const lower = input.toLowerCase().trim();
      if (lower.length < 100 && ON_DEVICE_TRIGGERS.some((t) => lower.includes(t))) {
        return "on_device";
      }
      return "cloud";
    },

    async inferIntent(input: string): Promise<string> {
      if (isOffline()) return placeholder.inferIntent(input);
      try {
        const res = await client.generate({
          model,
          prompt: `You are an intent classifier. Reply with ONLY one word: time, weather, calculator, greeting, echo, or general.\n\nClassify intent: "${input}"`,
          stream: false,
          options: { num_predict: 8 },
        });
        const text = (res.response ?? "").toLowerCase().replace(/[^a-z]/g, "") || "general";
        return ["time", "weather", "calculator", "greeting", "echo", "general"].includes(text)
          ? text
          : "general";
      } catch {
        return placeholder.inferIntent(input);
      }
    },

    async generateCloudResponse(input: string): Promise<string> {
      if (isOffline()) {
        return `[On-device] (offline) "${input.slice(0, 50)}..." — use built-in skills or enable network.`;
      }
      try {
        const res = await client.generate({
          model,
          prompt: `You are a helpful AI assistant. Reply concisely.\n\nUser: ${input}`,
          stream: false,
          options: { num_predict: 256 },
        });
        const text = (res.response ?? "").trim() || "(No response)";
        return `[On-device/Ollama] ${text}`;
      } catch (err) {
        return `[On-device] (Ollama unavailable) "${input.slice(0, 50)}..." — ${String(err)}`;
      }
    },
  };
}

async function checkOllama(client: Ollama): Promise<boolean> {
  try {
    await client.list();
    return true;
  } catch {
    return false;
  }
}
