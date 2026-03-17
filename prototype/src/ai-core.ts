/**
 * AIOS AI Core prototype
 *
 * - Accepts text input (Phase 1: text only)
 * - Routes to on-device vs cloud
 * - Returns intent + response (placeholder logic)
 */

import type { InferenceTarget } from "./types.js";

/** Simple intent classification (placeholder; no real model) */
const ON_DEVICE_TRIGGERS = [
  "time",
  "date",
  "weather",
  "calculator",
  "calc",
  "hello",
  "hi",
  "what time",
  "what's the time",
];

/** Route input to on-device or cloud */
export function route(input: string): InferenceTarget {
  const lower = input.toLowerCase().trim();
  if (lower.length < 100 && ON_DEVICE_TRIGGERS.some((t) => lower.includes(t))) {
    return "on_device";
  }
  return "cloud";
}

/** Infer intent (placeholder; would use LLM in production) */
export function inferIntent(input: string): string {
  const lower = input.toLowerCase().trim();
  if (lower.includes("time") || lower.includes("date")) return "time";
  if (lower.includes("weather")) return "weather";
  if (lower.includes("calc") || lower.includes("+") || lower.includes("*"))
    return "calculator";
  if (lower.includes("hello") || lower.includes("hi")) return "greeting";
  return "general";
}

/** Process text input and return AI response (prototype) */
export function process(input: string): {
  target: InferenceTarget;
  intent: string;
  message: string;
} {
  const target = route(input);
  const intent = inferIntent(input);

  // Placeholder responses (no real inference)
  let message: string;
  if (intent === "time") {
    const now = new Date();
    message = `[On-device] Current time: ${now.toLocaleTimeString()}`;
  } else if (intent === "greeting") {
    message = "[On-device] Hello! How can I help you?";
  } else if (intent === "calculator") {
    message =
      "[On-device] Calculator would evaluate here. (Not implemented in prototype.)";
  } else if (target === "on_device") {
    message = `[On-device] Intent: ${intent}. (Placeholder response.)`;
  } else {
    message = `[Cloud] Complex query routed to cloud: "${input.slice(0, 50)}..."`;
  }

  return { target, intent, message };
}
