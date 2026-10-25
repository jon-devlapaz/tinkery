// The Pi extension Tinkery loads with read-only tools: every tool call stays inside the folder Pi
// starts in. It refuses tools other than read, grep, find and ls; paths Pi would rewrite (a
// leading `~` or `@`, `file:` URLs, unusual spaces); paths that don't exist; and paths whose real
// location, after symlinks, is outside the folder. Pi blocks the call if this handler fails.
import { realpathSync } from 'node:fs';
import { isAbsolute, relative, resolve, sep } from 'node:path';

const TOOLS = new Set(['read', 'grep', 'find', 'ls']);
const REWRITTEN = /^[~@]|^file:|[  -   　]/;

export function refusal(tool, path, folder) {
  if (!TOOLS.has(tool)) return `${tool} is not available here.`;
  const asked = path || '.';
  if (typeof asked !== 'string' || REWRITTEN.test(asked)) return 'Use a plain path inside this folder.';
  let real, root;
  try { real = realpathSync(resolve(folder, asked)); root = realpathSync(folder); } catch { return `${asked} doesn't exist in this folder.`; }
  const inside = relative(root, real);
  if (inside === '' || (inside !== '..' && !inside.startsWith(`..${sep}`) && !isAbsolute(inside))) return undefined;
  return `${asked} is outside this folder. Only this folder can be read.`;
}

export default function (pi) {
  pi.on('tool_call', (event, ctx) => {
    const reason = refusal(event.toolName, event.input?.path, ctx?.cwd || process.cwd());
    return reason ? { block: true, reason } : undefined;
  });
}
