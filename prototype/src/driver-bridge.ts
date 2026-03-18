/**
 * Driver bridge — spawns aios-driver-bridge for hardware capture (camera, mic).
 * Requires Linux (V4L2, ALSA). On macOS or when binary missing, returns null.
 */

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

/** Path to driver-bridge binary (release build from project root) */
function getBridgePath(): string | null {
  const root = join(process.cwd(), "..");
  const candidates = [
    join(root, "target", "release", "aios-driver-bridge"),
    join(root, "target", "debug", "aios-driver-bridge"),
  ];
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  return null;
}

export interface CameraResult {
  ok: boolean;
  format?: string;
  data?: string;
  error?: string;
}

export interface AudioResult {
  ok: boolean;
  format?: string;
  sample_rate?: number;
  channels?: number;
  data?: string;
  error?: string;
}

/** Capture image from camera. Returns Buffer or null on failure. */
export function captureFromCamera(device = "/dev/video0"): Buffer | null {
  const bin = getBridgePath();
  if (!bin) return null;
  const r = spawnSync(bin, ["camera", "capture", device], {
    encoding: "utf8",
    maxBuffer: 50 * 1024 * 1024, // 50MB for large images
  });
  if (r.status !== 0) return null;
  try {
    const out = JSON.parse(r.stdout.trim()) as CameraResult;
    if (!out.ok || !out.data) return null;
    return Buffer.from(out.data, "base64");
  } catch {
    return null;
  }
}

/** Capture audio from microphone. Returns Buffer (PCM s16le) or null. */
export function captureFromMic(samples = 48000): Buffer | null {
  const bin = getBridgePath();
  if (!bin) return null;
  const r = spawnSync(bin, ["audio", "capture", String(samples)], {
    encoding: "utf8",
    maxBuffer: 10 * 1024 * 1024,
  });
  if (r.status !== 0) return null;
  try {
    const out = JSON.parse(r.stdout.trim()) as AudioResult;
    if (!out.ok || !out.data) return null;
    return Buffer.from(out.data, "base64");
  } catch {
    return null;
  }
}

/** Check if driver bridge is available (Linux with built binary). */
export function isDriverBridgeAvailable(): boolean {
  return getBridgePath() !== null;
}
