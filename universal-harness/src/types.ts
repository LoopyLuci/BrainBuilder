// The Capability Manifest is the one artifact every layer of this toolkit
// agrees on: introspectors (cli/api/gui) produce it, executors consume it,
// and the MCP server turns it into tools — so any of the three target kinds
// looks identical from an agent's point of view once introspected.

export type OperationKind = 'cli' | 'api' | 'gui';

export interface ParamSpec {
  /** Name used when calling the operation (the key in the args object). */
  name: string;
  description?: string;
  required: boolean;
  /** Best-effort type hint extracted during introspection; not enforced. */
  type?: 'string' | 'number' | 'boolean';
}

export interface CliOperation {
  kind: 'cli';
  name: string;
  description: string;
  /** Subcommand words inserted between the base command and flags, e.g. ["remote", "add"] for `git remote add`. */
  subcommandPath: string[];
  params: CliParamSpec[];
}

export interface CliParamSpec extends ParamSpec {
  /** The literal flag text (e.g. "--branch", "-b"). Absent for positional args. */
  flag?: string;
  /** True if the flag is a boolean switch that takes no value (e.g. "--verbose"). */
  isSwitch?: boolean;
}

export interface ApiOperation {
  kind: 'api';
  name: string;
  description: string;
  method: string;
  /** Path template, e.g. "/predict/{model_id}". */
  path: string;
  params: ApiParamSpec[];
  hasBody: boolean;
}

export interface ApiParamSpec extends ParamSpec {
  in: 'path' | 'query' | 'header';
}

export type GuiAction = 'click' | 'fill' | 'read' | 'exists';

export interface GuiOperation {
  kind: 'gui';
  name: string;
  description: string;
  action: GuiAction;
  selector: string;
  /** How the selector was derived — lower confidence means more likely to break across app updates. */
  selectorConfidence: 'stable' | 'best-effort';
  /** For 'fill': the param name whose value is typed into the element. */
  params: ParamSpec[];
}

export type Operation = CliOperation | ApiOperation | GuiOperation;

export interface CliTarget {
  kind: 'cli';
  /** The base command/binary, e.g. "git" or "node scripts/ci/pipeline.mjs". */
  command: string;
  cwd?: string;
}

export interface ApiTarget {
  kind: 'api';
  baseUrl: string;
  headers?: Record<string, string>;
}

export interface GuiTarget {
  kind: 'gui';
  /** Chrome DevTools Protocol HTTP endpoint, e.g. "http://localhost:9222". */
  cdpUrl: string;
  /** Substring matched against a page target's URL — required when the app exposes more than one window/tab over CDP. */
  urlIncludes?: string;
}

export type Target = CliTarget | ApiTarget | GuiTarget;

export interface CapabilityManifest {
  /** Human name of the software being harnessed, e.g. "BrainBuilder GUI". */
  name: string;
  description?: string;
  target: Target;
  generatedAt: string;
  operations: Operation[];
}

export interface ExecutionResult {
  ok: boolean;
  output?: unknown;
  error?: string;
  /** Unprocessed backing data (stdout/stderr, raw HTTP response, raw CDP result) for debugging. */
  raw?: unknown;
}
