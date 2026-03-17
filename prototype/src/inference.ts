/**
 * AIOS Pluggable Inference
 *
 * Abstraction for inference backends: placeholder, OpenAI, or Anthropic.
 * Set AIOS_INFERENCE=openai|anthropic|placeholder (default) to switch.
 * Requires OPENAI_API_KEY or ANTHROPIC_API_KEY for respective backends.
 */

import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";
import type { InferenceTarget } from "./types.js";

/** Inference backend interface */
export interface InferenceBackend {
  /** Route input to on-device or cloud */
  route(input: string): InferenceTarget;

  /** Infer intent from input (can use LLM) */
  inferIntent(input: string): Promise<string>;

  /** Generate response for cloud-routed queries (LLM call) */
  generateCloudResponse?(input: string): Promise<string>;
}

const ON_DEVICE_TRIGGERS = [
  "time", "date", "weather", "calculator", "calc",
  "hello", "hi", "what time", "what's the time", "echo",
];

/** Call LLM for intent classification. Returns single-word intent. */
const INTENT_SYSTEM = "You are an intent classifier. Reply with ONLY one word: time, weather, calculator, greeting, echo, or general.";

/** Placeholder: keyword-based routing and intent (no LLM) */
function createPlaceholderBackend(): InferenceBackend {
  return {
    route(input: string): InferenceTarget {
      const lower = input.toLowerCase().trim();
      if (lower.length < 100 && ON_DEVICE_TRIGGERS.some((t) => lower.includes(t))) {
        return "on_device";
      }
      return "cloud";
    },

    async inferIntent(input: string): Promise<string> {
      const lower = input.toLowerCase().trim();
      if (lower.startsWith("echo ") || lower === "echo") return "echo";
      if (lower.includes("time") || lower.includes("date")) return "time";
      if (lower.includes("weather")) return "weather";
      if (lower.includes("calc") || lower.includes("+") || lower.includes("*")) return "calculator";
      if (lower.includes("hello") || lower.includes("hi")) return "greeting";
      return "general";
    },

    generateCloudResponse: async (input: string) => {
      return `[Cloud] Complex query routed to cloud: "${input.slice(0, 50)}..."`;
    },
  };
}

/** OpenAI backend (requires OPENAI_API_KEY) */
function createOpenAIBackend(): InferenceBackend {
  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey) {
    console.warn("OPENAI_API_KEY not set, falling back to placeholder");
    return createPlaceholderBackend();
  }

  const client = new OpenAI({ apiKey });
  const model = process.env.AIOS_OPENAI_MODEL ?? "gpt-4o-mini";

  const callChat = async (messages: OpenAI.Chat.Completions.ChatCompletionMessageParam[]) => {
    const res = await client.chat.completions.create({
      model,
      messages,
      max_tokens: 256,
    });
    return res.choices[0]?.message?.content?.trim() ?? "";
  };

  return {
    route(input: string): InferenceTarget {
      const lower = input.toLowerCase().trim();
      if (lower.length < 100 && ON_DEVICE_TRIGGERS.some((t) => lower.includes(t))) {
        return "on_device";
      }
      return "cloud";
    },

    async inferIntent(input: string): Promise<string> {
      try {
        const text = await callChat([
          { role: "system", content: INTENT_SYSTEM },
          { role: "user", content: `Classify intent: "${input}"` },
        ]);
        return text.toLowerCase().replace(/[^a-z]/g, "") || "general";
      } catch {
        return createPlaceholderBackend().inferIntent(input);
      }
    },

    async generateCloudResponse(input: string): Promise<string> {
      try {
        const text = await callChat([{ role: "user", content: input }]);
        return `[Cloud/OpenAI] ${text}`;
      } catch (err) {
        return `[Error] ${String(err)}`;
      }
    },
  };
}

/** Anthropic backend (requires ANTHROPIC_API_KEY) */
function createAnthropicBackend(): InferenceBackend {
  const apiKey = process.env.ANTHROPIC_API_KEY;
  if (!apiKey) {
    console.warn("ANTHROPIC_API_KEY not set, falling back to placeholder");
    return createPlaceholderBackend();
  }

  const client = new Anthropic({ apiKey });
  const model = process.env.AIOS_ANTHROPIC_MODEL ?? "claude-3-5-haiku-20241022";

  const callMessages = async (system: string, user: string) => {
    const res = await client.messages.create({
      model,
      max_tokens: 256,
      system,
      messages: [{ role: "user", content: user }],
    });
    const block = res.content.find((b) => b.type === "text");
    return block && "text" in block ? block.text.trim() : "";
  };

  return {
    route(input: string): InferenceTarget {
      const lower = input.toLowerCase().trim();
      if (lower.length < 100 && ON_DEVICE_TRIGGERS.some((t) => lower.includes(t))) {
        return "on_device";
      }
      return "cloud";
    },

    async inferIntent(input: string): Promise<string> {
      try {
        const text = await callMessages(INTENT_SYSTEM, `Classify intent: "${input}"`);
        return text.toLowerCase().replace(/[^a-z]/g, "") || "general";
      } catch {
        return createPlaceholderBackend().inferIntent(input);
      }
    },

    async generateCloudResponse(input: string): Promise<string> {
      try {
        const text = await callMessages("You are a helpful AI assistant.", input);
        return `[Cloud/Anthropic] ${text}`;
      } catch (err) {
        return `[Error] ${String(err)}`;
      }
    },
  };
}

let cachedBackend: InferenceBackend | null = null;

/** Resolve backend from env. Caches result. */
export function getInferenceBackend(): InferenceBackend {
  if (cachedBackend) return cachedBackend;

  const mode = (process.env.AIOS_INFERENCE ?? "placeholder").toLowerCase();
  cachedBackend =
    mode === "openai"
      ? createOpenAIBackend()
      : mode === "anthropic"
        ? createAnthropicBackend()
        : createPlaceholderBackend();

  return cachedBackend;
}
