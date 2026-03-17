/**
 * AIOS UI helpers — colored output for terminal
 */

import chalk from "chalk";

export const c = {
  brand: chalk.cyan.bold,
  success: chalk.green,
  cloud: chalk.blue,
  onDevice: chalk.green.dim,
  tool: chalk.yellow.dim,
  error: chalk.red,
  dim: chalk.dim,
};

export function formatResponse(result: { target: string; intent?: string; message: string; toolsUsed?: string[] }): string {
  const targetLabel = result.target === "on_device" ? c.onDevice("[On-device]") : c.cloud("[Cloud]");
  const toolsStr = result.toolsUsed?.length ? ` ${c.tool(`[${result.toolsUsed.join(", ")}]`)}` : "";
  return `${targetLabel} ${result.message}${toolsStr}`;
}
