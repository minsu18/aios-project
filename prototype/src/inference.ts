/**
 * AIOS Pluggable Inference
 *
 * Abstraction for inference backends: placeholder, on-device, or cloud LLM.
 * Set AIOS_INFERENCE=openai|anthropic|placeholder (default) to switch.
 */

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

/** Placeholder: keyword-based routing and intent (no LLM) */
function createPlaceholderBackend(): InferenceBackend {
  const ON_DEVICE_TRIGGERS = [
    "time", "date", "weather", "calculator", "calc",
    "hello", "hi", "what time", "what's the time", "echo",
  ];

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

/** Resolve backend from env (future: openai, anthropic) */
export function getInferenceBackend(): InferenceBackend {
  const mode = (process.env.AIOS_INFERENCE ?? "placeholder").toLowerCase();
  switch (mode) {
    case "openai":
      // Future: return createOpenAIBackend();
      console.warn("AIOS_INFERENCE=openai not yet implemented, falling back to placeholder");
      return createPlaceholderBackend();
    case "anthropic":
      // Future: return createAnthropicBackend();
      console.warn("AIOS_INFERENCE=anthropic not yet implemented, falling back to placeholder");
      return createPlaceholderBackend();
    default:
      return createPlaceholderBackend();
  }
}
