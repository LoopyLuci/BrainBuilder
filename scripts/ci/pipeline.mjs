#!/usr/bin/env node
// BrainBuilder's CI/CD pipeline — runs entirely on this machine, no cloud
// runner, no GitHub dependency. Plain Node.js (already a required dependency
// via the GUI's own toolchain) is the one thing guaranteed identical across
// Windows/macOS/Linux, so this one script *is* the cross-platform story:
// the same file, unmodified, drives the same stages on every OS a
// contributor or user actually has. `.github/workflows/*.yml` remain as an
// optional cloud mirror for anyone who wants GitHub's hosted runners too, but
// this is the real, required system — it works with zero network access and
// zero third-party account.
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..", "..");
const GUI = join(ROOT, "gui");

const args = new Set(process.argv.slice(2));
const FULL = args.has("--full");
const PACKAGE = args.has("--package");
const INSTALL_HOOKS = args.has("--install-hooks");
const HELP = args.has("--help") || args.has("-h");

const USAGE = `pipeline.mjs — BrainBuilder's local CI/CD pipeline (no GitHub dependency)

Usage: node scripts/ci/pipeline.mjs [options]

Options:
  --full            Also run rust-test-full (needs python+torch on PATH)
  --package         Also run package (builds a native installer for this OS)
  --install-hooks   Install a git pre-push hook that runs this pipeline, then exit
  --help, -h        Show this help
`;

if (HELP) {
  process.stdout.write(USAGE);
  process.exit(0);
}

/** @type {{name: string, cwd: string, cmd: string, cmdArgs: string[], when: boolean}[]} */
const stages = [
  {
    name: "rust-build",
    cwd: ROOT,
    cmd: "cargo",
    cmdArgs: ["build", "--workspace", "--locked"],
    when: true,
  },
  {
    name: "rust-test-fast",
    cwd: ROOT,
    cmd: "cargo",
    cmdArgs: ["test", "-p", "brainbuilder-core", "--lib", "--locked"],
    when: true,
    about: "Pure-Rust unit tests — no Python/torch/Racket/Clojure needed, so this runs unconditionally on every machine.",
  },
  {
    name: "frontend-typecheck",
    cwd: GUI,
    cmd: "npx",
    cmdArgs: ["tsc", "--noEmit"],
    when: true,
  },
  {
    name: "frontend-build",
    cwd: GUI,
    cmd: "npm",
    cmdArgs: ["run", "build"],
    when: true,
  },
  {
    name: "rust-test-full",
    cwd: ROOT,
    cmd: "cargo",
    cmdArgs: [
      "test", "-p", "brainbuilder-core", "--locked",
      // This package has ~30 separate integration-test binaries, each
      // linking the full dependency tree (datafusion, libp2p, arrow, tokio,
      // ...). Cargo's default job count (one per logical core) launches
      // that many linker processes at once, which reserves far more virtual
      // address space than a modest fixed-size Windows page file can back —
      // rustc/link.exe then crash outright (STATUS_STACK_BUFFER_OVERRUN) or
      // leave a truncated rlib that the next test binary fails to mmap
      // ("paging file is too small", E0786). This isn't flaky test *code* —
      // it's this build's memory footprint outrunning the page file at the
      // default parallelism. Capping jobs keeps peak concurrent linker
      // memory bounded regardless of core count.
      "-j", "4",
      "--", "--include-ignored",
      // These two need racket/clojure/a JDK on PATH — a real, separate
      // toolchain requirement most machines (including CI-style ones)
      // won't have; skipped here, not silently passed.
      "--skip", "racket_symbolic_diff_roundtrip",
      "--skip", "clojure_defgraph_roundtrip",
    ],
    when: FULL,
    about: "Needs `python`+`torch` on PATH. Pass --full to run this (e.g. before a release).",
  },
  {
    name: "package",
    cwd: GUI,
    cmd: "npx",
    cmdArgs: ["tauri", "build"],
    when: PACKAGE,
    about: "Builds a real, native installer for *this* OS only (msi/nsis on Windows, dmg on macOS, deb/AppImage on Linux) — run it on each OS you want to ship for, no cross-compilation, no cloud runner.",
  },
];

function runStage(stage) {
  const start = Date.now();
  process.stdout.write(`\n▶ ${stage.name}${stage.about ? ` — ${stage.about}` : ""}\n`);
  // All stage commands/args are static and defined above (never derived from
  // user input), so building one shell string is safe here; doing it this
  // way — rather than `spawnSync(cmd, args, {shell: true})` — avoids Node's
  // args-with-shell:true footgun (each arg would otherwise be concatenated
  // unescaped rather than passed as a distinct argument).
  const commandLine = [stage.cmd, ...stage.cmdArgs].join(" ");
  const result = spawnSync(commandLine, {
    cwd: stage.cwd,
    stdio: "inherit",
    shell: true,
  });
  const durationMs = Date.now() - start;
  const ok = result.status === 0 && !result.error;
  return {
    name: stage.name,
    ok,
    durationMs,
    error: result.error ? String(result.error) : null,
    exitCode: result.status,
  };
}

function installGitHook() {
  const gitDir = join(ROOT, ".git");
  // `mkdirSync(..., {recursive: true})` happily creates `.git` itself if
  // missing — it doesn't verify a real repo exists, only that the path
  // doesn't. Checking for a real marker (`.git/HEAD`, present the instant
  // `git init` runs) before touching anything avoids fabricating a
  // look-alike `.git/hooks` directory with no actual repository behind it.
  if (!existsSync(join(gitDir, "HEAD"))) {
    console.error("No git repository found here — run `git init` first, then re-run with --install-hooks.");
    process.exitCode = 1;
    return;
  }
  const hooksDir = join(gitDir, "hooks");
  mkdirSync(hooksDir, { recursive: true });
  const hookPath = join(hooksDir, "pre-push");
  // Git always runs hooks through `sh`, even on Windows (Git for Windows
  // ships one for exactly this reason) — so a POSIX shell script here is the
  // actually-cross-platform choice, not a bash-vs-PowerShell fork.
  const hookBody = `#!/bin/sh\n# Installed by scripts/ci/pipeline.mjs --install-hooks\nnode "${join(ROOT, "scripts", "ci", "pipeline.mjs").replace(/\\/g, "/")}"\n`;
  writeFileSync(hookPath, hookBody, { mode: 0o755 });
  console.log(`Installed pre-push hook at ${hookPath} — every \`git push\` now runs this pipeline first.`);
}

function main() {
  if (INSTALL_HOOKS) {
    installGitHook();
    return;
  }

  const results = [];
  for (const stage of stages) {
    if (!stage.when) {
      results.push({ name: stage.name, ok: null, durationMs: 0, skipped: true });
      continue;
    }
    const result = runStage(stage);
    results.push(result);
    if (!result.ok) break; // fail fast, same as a real CI pipeline
  }

  console.log("\n" + "=".repeat(60));
  console.log("BrainBuilder CI/CD summary");
  console.log("=".repeat(60));
  for (const r of results) {
    const label = r.skipped ? "SKIPPED" : r.ok ? "PASS" : "FAIL";
    const time = r.skipped ? "" : ` (${(r.durationMs / 1000).toFixed(1)}s)`;
    console.log(`  [${label}]${time} ${r.name}`);
  }

  const reportsDir = join(ROOT, "ci-reports");
  mkdirSync(reportsDir, { recursive: true });
  const reportPath = join(reportsDir, `${new Date().toISOString().replace(/[:.]/g, "-")}.json`);
  writeFileSync(reportPath, JSON.stringify({ ranAt: new Date().toISOString(), full: FULL, package: PACKAGE, results }, null, 2));
  console.log(`\nReport written to ${reportPath}`);

  const failed = results.some((r) => r.ok === false);
  process.exitCode = failed ? 1 : 0;
}

main();
