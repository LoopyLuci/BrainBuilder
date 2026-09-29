// Heuristic --help parser. There is no universal CLI help format, so this
// targets the conventions that cover the overwhelming majority of real
// tools: GNU-style long/short flags, argparse-style, and a "Commands:" /
// "Available Commands:" section for subcommands (git/npm/cargo/docker style).
// It is deliberately best-effort — selectorConfidence-style honesty belongs
// here too, so every parsed param is a guess, not a guarantee. Verify a
// generated manifest against the real tool before trusting it blindly.
import { spawn } from 'node:child_process';
import path from 'node:path';
import type { CapabilityManifest, CliOperation, CliParamSpec } from '../types.js';

export interface CliIntrospectOptions {
  /** Base command, e.g. "git" or "node scripts/ci/pipeline.mjs". Split on whitespace. */
  command: string;
  cwd?: string;
  /** Max recursion depth into subcommands (0 = don't recurse). Default 1. */
  maxDepth?: number;
  /** Flag used to request help text. Default "--help". */
  helpFlag?: string;
  timeoutMs?: number;
}

function splitCommand(command: string): string[] {
  return command.trim().split(/\s+/).filter(Boolean);
}

async function runHelp(commandParts: string[], subPath: string[], helpFlag: string, cwd: string | undefined, timeoutMs: number): Promise<string> {
  return new Promise((resolve) => {
    const [bin, ...baseArgs] = commandParts;
    const child = spawn(bin, [...baseArgs, ...subPath, helpFlag], { cwd, shell: false });
    let out = '';
    const timer = setTimeout(() => {
      child.kill();
      resolve(out);
    }, timeoutMs);
    child.stdout.on('data', (d) => (out += d.toString()));
    child.stderr.on('data', (d) => (out += d.toString()));
    child.on('error', () => {
      clearTimeout(timer);
      resolve(out);
    });
    child.on('close', () => {
      clearTimeout(timer);
      resolve(out);
    });
  });
}

const FLAG_LINE_PATTERNS: RegExp[] = [
  // -f, --flag <VALUE>   Description   (short + long, optional value)
  /^\s*(-\w),\s*(--[\w-]+)(?:[ =]<?([\w.:|-]+)>?)?\s{2,}(.*)$/,
  // --flag <VALUE>   Description   (long only)
  /^\s*(--[\w-]+)(?:[ =]<?([\w.:|-]+)>?)?\s{2,}(.*)$/,
  // -f VALUE   Description   (short only)
  /^\s*(-\w)(?:\s+([A-Z][\w.:-]*))?\s{2,}(.*)$/,
];

function parseFlagLine(line: string): CliParamSpec | null {
  for (const pattern of FLAG_LINE_PATTERNS) {
    const m = line.match(pattern);
    if (!m) continue;
    if (pattern === FLAG_LINE_PATTERNS[0]) {
      const [, short, long, value, description] = m;
      return {
        name: long.replace(/^--/, ''),
        flag: long,
        description: description?.trim(),
        required: false,
        isSwitch: !value,
        type: value ? 'string' : 'boolean',
      };
    }
    if (pattern === FLAG_LINE_PATTERNS[1]) {
      const [, long, value, description] = m;
      return {
        name: long.replace(/^--/, ''),
        flag: long,
        description: description?.trim(),
        required: false,
        isSwitch: !value,
        type: value ? 'string' : 'boolean',
      };
    }
    const [, short, value, description] = m;
    return {
      name: short.replace(/^-/, ''),
      flag: short,
      description: description?.trim(),
      required: false,
      isSwitch: !value,
      type: value ? 'string' : 'boolean',
    };
  }
  return null;
}

const SECTION_HEADER = /^([A-Za-z][A-Za-z /]*):\s*$/;
const COMMANDS_HEADER = /^(commands|available commands|subcommands)$/i;

interface ParsedHelp {
  params: CliParamSpec[];
  subcommands: { name: string; description: string }[];
  description: string;
}

function parseHelpText(text: string): ParsedHelp {
  const lines = text.split(/\r?\n/);
  const params: CliParamSpec[] = [];
  const subcommands: { name: string; description: string }[] = [];
  let section: 'commands' | 'other' | null = null;
  let description = '';

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const headerMatch = line.match(SECTION_HEADER);
    if (headerMatch) {
      section = COMMANDS_HEADER.test(headerMatch[1].trim()) ? 'commands' : 'other';
      continue;
    }
    if (!line.trim()) {
      continue;
    }
    if (section === 'commands') {
      const m = line.match(/^\s*([\w:.-]+)\s{2,}(.*)$/);
      if (m) subcommands.push({ name: m[1], description: m[2].trim() });
      continue;
    }
    const flag = parseFlagLine(line);
    if (flag) {
      params.push(flag);
      continue;
    }
    if (!description && i < 5 && !line.trim().toLowerCase().startsWith('usage')) {
      description = line.trim();
    }
  }
  return { params, subcommands, description };
}

export async function introspectCli(options: CliIntrospectOptions): Promise<CapabilityManifest> {
  const commandParts = splitCommand(options.command);
  const maxDepth = options.maxDepth ?? 1;
  const helpFlag = options.helpFlag ?? '--help';
  const timeoutMs = options.timeoutMs ?? 10_000;
  const operations: CliOperation[] = [];

  async function walk(subPath: string[], depth: number) {
    const helpText = await runHelp(commandParts, subPath, helpFlag, options.cwd, timeoutMs);
    const parsed = parseHelpText(helpText);
    const lastPart = commandParts[commandParts.length - 1];
    const leafName = subPath.length ? subPath.join(' ') : (lastPart ? path.basename(lastPart, path.extname(lastPart)) : 'root');

    if (parsed.subcommands.length > 0 && depth < maxDepth) {
      for (const sub of parsed.subcommands) {
        await walk([...subPath, sub.name], depth + 1);
      }
      return;
    }

    operations.push({
      kind: 'cli',
      name: leafName.replace(/[^\w.-]+/g, '-'),
      description: parsed.description || leafName,
      subcommandPath: subPath,
      params: parsed.params,
    });
  }

  await walk([], 0);

  return {
    name: options.command,
    description: `CLI tool introspected from \`${options.command} ${helpFlag}\``,
    target: { kind: 'cli', command: options.command, cwd: options.cwd },
    generatedAt: new Date().toISOString(),
    operations,
  };
}
