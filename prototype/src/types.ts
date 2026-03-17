/**
 * Shared types for AIOS prototype.
 */

/** Input modality */
export type InputModality = "text" | "voice" | "image" | "video";

/** Multimodal input payload */
export interface MultimodalInput {
  modality: InputModality;
  /** Text content or transcript */
  text?: string;
  /** Voice: WAV bytes (Base64 or Buffer) */
  voice?: Buffer | string;
  /** Image: raw bytes or base64 */
  image?: Buffer | string;
  /** Image prompt (optional; used when modality is image) */
  imagePrompt?: string;
  /** Video: raw bytes or base64 */
  video?: Buffer | string;
}

/** Inference target (routing decision) */
export type InferenceTarget = "on_device" | "cloud";

/** MCP-compatible tool definition (subset of MCP spec) */
export interface MCPTool {
  name: string;
  description: string;
  inputSchema: {
    type: "object";
    properties?: Record<string, { type: string; description?: string }>;
    required?: string[];
  };
}

/** Permission scope for skill sandbox */
export type SkillPermission = "network" | "filesystem" | "env";

/** Skill metadata from SKILL.md frontmatter */
export interface SkillMeta {
  name: string;
  description: string;
  version: string;
  author?: string;
  category?: string;
  permissions?: SkillPermission[];
  tools?: MCPTool[];
}

/** Parsed skill (meta + body) */
export interface Skill {
  meta: SkillMeta;
  body: string;
  path: string;
}

/** AI Core response */
export interface AIResponse {
  target: InferenceTarget;
  intent?: string;
  message: string;
  toolsUsed?: string[];
}

/** Tool handler: (args) => result (sync or async) */
export type ToolHandler = (args: Record<string, unknown>) => unknown | Promise<unknown>;
