/**
 * AIOS AI Core prototype
 *
 * - Accepts text input (Phase 1: text only; multimodal API ready)
 * - Routes to on-device vs cloud via pluggable inference
 * - Invokes skill tools when intent matches
 */

import type { InferenceTarget, Skill, AIResponse, MultimodalInput } from "./types.js";
import { invokeTool } from "./skill-runtime.js";
import { listTools } from "./skill-runtime.js";
import { getInferenceBackend } from "./inference.js";

/** Route input to on-device or cloud (delegates to inference backend) */
export function route(input: string): InferenceTarget {
  return getInferenceBackend().route(input);
}

/** Infer intent (delegates to inference backend; may use LLM) */
export async function inferIntent(input: string): Promise<string> {
  return getInferenceBackend().inferIntent(input);
}

/** Map intent + input to tool call (tool name, args) or null */
function selectTool(
  intent: string,
  input: string,
  skills: Skill[]
): { tool: string; args: Record<string, unknown> } | null {
  const tools = listTools(skills);
  const hasTool = (name: string) => tools.some((t) => t.name === name);

  if (intent === "time" && hasTool("example.get_time")) {
    // Match "time in Tokyo" or "timezone America/New_York" (avoid "time is" -> "is")
    const tzMatch = input.match(/(?:time\s+in|timezone)\s+([\w/]+)/i);
    return {
      tool: "example.get_time",
      args: tzMatch ? { timezone: tzMatch[1] } : {},
    };
  }
  if (intent === "echo" && hasTool("example.echo")) {
    const text = input.replace(/^echo\s+/i, "").trim() || input;
    return { tool: "example.echo", args: { text } };
  }
  return null;
}

/**
 * Process text input and return AI response.
 * Invokes skill tools when intent matches; otherwise uses inference backend.
 */
export async function process(
  input: string,
  skills: Skill[] = []
): Promise<AIResponse> {
  const backend = getInferenceBackend();
  const target = backend.route(input);
  const intent = await backend.inferIntent(input);

  const toolCall = selectTool(intent, input, skills);
  if (toolCall) {
    const result = await invokeTool(skills, toolCall.tool, toolCall.args);
    if (result.error) {
      return {
        target,
        intent,
        message: `[Error] ${result.error}`,
        toolsUsed: [toolCall.tool],
      };
    }
    const text = result.content
      .map((c) => (typeof c === "object" && c && "text" in c ? (c as { text: string }).text : String(c)))
      .join("");
    return {
      target,
      intent,
      message: `[On-device] ${text}`,
      toolsUsed: [toolCall.tool],
    };
  }

  // Fallback: placeholder responses
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
  } else if (backend.generateCloudResponse) {
    message = await backend.generateCloudResponse(input);
  } else {
    message = `[Cloud] Complex query routed to cloud: "${input.slice(0, 50)}..."`;
  }

  return { target, intent, message };
}

/**
 * Extract text from multimodal input. Uses STT for voice, Vision for image.
 */
async function extractTextFromMultimodal(input: MultimodalInput): Promise<string> {
  if (input.text) return input.text;
  if (input.modality === "voice" && input.voice) {
    try {
      const { transcribeVoice } = await import("./multimodal.js");
      return await transcribeVoice(input.voice);
    } catch (err) {
      return `[Voice STT error: ${String(err)}]`;
    }
  }
  if (input.modality === "image" && input.image) {
    try {
      const { describeImage } = await import("./multimodal.js");
      const prompt = input.imagePrompt ?? "Describe this image in detail.";
      return await describeImage(input.image, prompt);
    } catch (err) {
      return `[Image vision error: ${String(err)}]`;
    }
  }
  if (input.modality === "video") {
    return "[Video input - Multimodal pipeline not yet implemented. Use image or voice.]";
  }
  return "";
}

/**
 * Process multimodal input. Text/voice/image supported via STT and Vision APIs.
 */
export async function processMultimodal(
  input: MultimodalInput,
  skills: Skill[] = []
): Promise<AIResponse> {
  const text = await extractTextFromMultimodal(input);
  if (!text || text.startsWith("[")) {
    return {
      target: "cloud",
      intent: "general",
      message: text || "No text input provided.",
    };
  }
  return process(text, skills);
}
