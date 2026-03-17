/**
 * AIOS Transformers.js inference backend
 *
 * Runs local LLM via @huggingface/transformers (ONNX models).
 * Set AIOS_INFERENCE=transformers. Optionally AIOS_TRANSFORMERS_MODEL=Xenova/LaMini-Flan-T5-783M.
 * First run downloads the model (~300MB); subsequent runs use cache. Works offline once cached.
 */

import type { InferenceBackend } from "./inference.js";
import type { InferenceTarget } from "./types.js";
import { createPlaceholderBackend } from "./inference.js";
import { isOffline } from "./inference.js";

const ON_DEVICE_TRIGGERS = [
  "time", "date", "weather", "calculator", "calc",
  "hello", "hi", "what time", "what's the time", "echo",
];

const DEFAULT_MODEL = "Xenova/LaMini-Flan-T5-783M";

function getGeneratedText(out: unknown): string {
  const arr = Array.isArray(out) ? out : [out];
  const first = arr[0];
  if (first && typeof first === "object" && "generated_text" in first) {
    return String(first.generated_text ?? "");
  }
  return "";
}

/** Create Transformers.js backend. Falls back to placeholder on load/generate errors. */
export async function createTransformersBackend(): Promise<InferenceBackend> {
  const placeholder = createPlaceholderBackend();
  let pipe: ((input: string, opts?: object) => Promise<unknown>) | null = null;

  try {
    const mod = await import("@huggingface/transformers");
    const model = process.env.AIOS_TRANSFORMERS_MODEL ?? DEFAULT_MODEL;
    const p = await mod.pipeline("text2text-generation", model);
    pipe = p as (input: string, opts?: object) => Promise<unknown>;
  } catch (err) {
    console.warn(`Transformers.js failed to load model: ${String(err)}. Falling back to placeholder.`);
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
      if (isOffline() || !pipe) return placeholder.inferIntent(input);
      try {
        const out = await pipe(
          `Classify the intent as one word: time, weather, calculator, greeting, echo, or general. Input: "${input}"`,
          { max_new_tokens: 16, do_sample: false },
        );
        const text = getGeneratedText(out).toLowerCase().replace(/[^a-z]/g, "") || "general";
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
      if (!pipe) return placeholder.generateCloudResponse!(input);
      try {
        const prompt = `Answer briefly: ${input}`;
        const out = await pipe(prompt, {
          max_new_tokens: 256,
          temperature: 0.7,
          do_sample: true,
        });
        const text = getGeneratedText(out).trim() || "(No response)";
        return `[On-device/Transformers] ${text}`;
      } catch (err) {
        return `[On-device] (Transformers error) "${input.slice(0, 50)}..." — ${String(err)}`;
      }
    },
  };
}
