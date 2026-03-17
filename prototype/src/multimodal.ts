/**
 * AIOS Multimodal I/O — STT (Whisper) and Vision
 *
 * Voice -> text via OpenAI Whisper.
 * Image -> text via OpenAI GPT-4o or Anthropic Claude vision.
 * Requires OPENAI_API_KEY or ANTHROPIC_API_KEY.
 */

import OpenAI from "openai";
import { toFile } from "openai/uploads";
import Anthropic from "@anthropic-ai/sdk";

/** Convert Buffer or base64 string to Buffer */
function toBuffer(data: Buffer | string): Buffer {
  if (Buffer.isBuffer(data)) return data;
  return Buffer.from(data, "base64");
}

/**
 * Transcribe voice/audio to text using OpenAI Whisper.
 * Accepts WAV, MP3, M4A, etc.
 */
export async function transcribeVoice(audio: Buffer | string): Promise<string> {
  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey) {
    throw new Error("OPENAI_API_KEY not set. Required for voice transcription.");
  }

  const buffer = toBuffer(audio);
  const client = new OpenAI({ apiKey });
  const file = await toFile(buffer, "audio.wav", { type: "audio/wav" });

  const res = await client.audio.transcriptions.create({
    file,
    model: "whisper-1",
  });

  return typeof res === "string" ? res : (res as { text: string }).text ?? "";
}

/**
 * Describe image or answer question about it using vision API.
 * Uses OpenAI GPT-4o by default; falls back to Anthropic if no OpenAI key.
 */
export async function describeImage(
  image: Buffer | string,
  prompt: string = "Describe this image in detail."
): Promise<string> {
  const buffer = toBuffer(image);
  const base64 = buffer.toString("base64");

  if (process.env.OPENAI_API_KEY) {
    return describeImageOpenAI(base64, prompt);
  }
  if (process.env.ANTHROPIC_API_KEY) {
    return describeImageAnthropic(base64, prompt);
  }
  throw new Error("OPENAI_API_KEY or ANTHROPIC_API_KEY required for image input.");
}

async function describeImageOpenAI(base64: string, prompt: string): Promise<string> {
  const client = new OpenAI({ apiKey: process.env.OPENAI_API_KEY! });
  const model = process.env.AIOS_OPENAI_VISION_MODEL ?? "gpt-4o-mini";

  const res = await client.chat.completions.create({
    model,
    messages: [
      {
        role: "user",
        content: [
          { type: "text", text: prompt },
          {
            type: "image_url",
            image_url: { url: `data:image/png;base64,${base64}` },
          },
        ],
      },
    ],
    max_tokens: 512,
  });

  return res.choices[0]?.message?.content?.trim() ?? "";
}

async function describeImageAnthropic(base64: string, prompt: string): Promise<string> {
  const client = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY! });
  const model = process.env.AIOS_ANTHROPIC_VISION_MODEL ?? "claude-3-5-haiku-20241022";

  const res = await client.messages.create({
    model,
    max_tokens: 512,
    messages: [
      {
        role: "user",
        content: [
          { type: "image", source: { type: "base64", media_type: "image/png", data: base64 } },
          { type: "text", text: prompt },
        ],
      },
    ],
  });

  const block = res.content.find((b) => b.type === "text");
  return block && "text" in block ? block.text.trim() : "";
}
