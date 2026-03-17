/**
 * Shared types for AIOS prototype.
 */

/** Input modality */
export type InputModality = "text" | "voice" | "image" | "video";

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

/** Skill metadata from SKILL.md frontmatter */
export interface SkillMeta {
  name: string;
  description: string;
  version: string;
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
