/**
 * Whether the camera supports `command`, according to its `commandList`.
 * Unknown (no list, or a list that does not even name the basic commands
 * V360Lab uses) counts as supported, so nothing is hidden by mistake.
 */
export function commandSupported(supported: string[] | null, command: string): boolean {
  if (!supported || !supported.includes("status")) return true;
  return supported.includes(command);
}
