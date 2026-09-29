#!/usr/bin/env node
// A tiny CLI used only by universal-harness's own tests. Deliberately mimics
// common real-world conventions (git-style subcommands, GNU-style flags) so
// the introspector's parser is exercised against a known, stable target
// instead of a real tool's help text, which can change out from under tests.
const args = process.argv.slice(2);
const [sub, ...rest] = args;

function helpRoot() {
  return `fixture-cli: a test double for universal-harness

Usage: fixture-cli <command> [options]

Commands:
  greet       Print a greeting
  add         Add two numbers

Options:
  -h, --help  Show help
`;
}

function helpGreet() {
  return `Print a greeting

Usage: fixture-cli greet [options]

Options:
  -n, --name <NAME>  Who to greet
  -l, --loud         Shout the greeting
`;
}

function helpAdd() {
  return `Add two numbers

Usage: fixture-cli add [options]

Options:
  -a, --a <N>  First number
  -b, --b <N>  Second number
`;
}

if (!sub || sub === '--help' || sub === '-h') {
  process.stdout.write(helpRoot());
  process.exit(0);
}

if (rest.includes('--help') || rest.includes('-h')) {
  if (sub === 'greet') process.stdout.write(helpGreet());
  else if (sub === 'add') process.stdout.write(helpAdd());
  else process.stdout.write(helpRoot());
  process.exit(0);
}

if (sub === 'greet') {
  const nameIdx = rest.findIndex((a) => a === '--name' || a === '-n');
  const name = nameIdx >= 0 ? rest[nameIdx + 1] : 'world';
  const loud = rest.includes('--loud') || rest.includes('-l');
  const greeting = `Hello, ${name}!`;
  process.stdout.write(loud ? greeting.toUpperCase() : greeting);
  process.exit(0);
}

if (sub === 'add') {
  const aIdx = rest.findIndex((a) => a === '--a' || a === '-a');
  const bIdx = rest.findIndex((a) => a === '--b' || a === '-b');
  const a = aIdx >= 0 ? Number(rest[aIdx + 1]) : NaN;
  const b = bIdx >= 0 ? Number(rest[bIdx + 1]) : NaN;
  if (Number.isNaN(a) || Number.isNaN(b)) {
    process.stderr.write('error: --a and --b are required numbers\n');
    process.exit(1);
  }
  process.stdout.write(String(a + b));
  process.exit(0);
}

process.stderr.write(`unknown command: ${sub}\n`);
process.exit(1);
