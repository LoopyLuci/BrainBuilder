#!/usr/bin/env python3
"""
OmniForge Concierge – restricted Python sandbox.

Executes user-supplied (or agent-generated) Python code with a severely
limited set of builtins and a blocked import list. Captures stdout/stderr
and returns them to the caller.

Hardened against common escape techniques:
  - restricted __builtins__
  - blocked dangerous modules
  - AST rejection of eval/exec/compile/open/input and dunder attribute chains
  - no fallback to unrestricted execution
"""

from __future__ import annotations

import ast
import io
import sys
import traceback
from typing import Any, Dict, Set


# ---------------------------------------------------------------------------
# Safe environment
# ---------------------------------------------------------------------------

SAFE_BUILTINS: Dict[str, Any] = {
    "print": print,
    "len": len,
    "range": range,
    "int": int,
    "float": float,
    "str": str,
    "list": list,
    "dict": dict,
    "tuple": tuple,
    "set": set,
    "bool": bool,
    "True": True,
    "False": False,
    "None": None,
    "abs": abs,
    "min": min,
    "max": max,
    "sum": sum,
    "round": round,
    "enumerate": enumerate,
    "zip": zip,
    "map": map,
    "filter": filter,
    "sorted": sorted,
    "reversed": reversed,
    "isinstance": isinstance,
    "type": type,
    "hasattr": hasattr,
    "getattr": getattr,
    "setattr": setattr,
    "repr": repr,
    "ascii": ascii,
    "ord": ord,
    "chr": chr,
    "hex": hex,
    "bin": bin,
    "oct": oct,
    "pow": pow,
    "divmod": divmod,
    "all": all,
    "any": any,
    "iter": iter,
    "next": next,
    "slice": slice,
    "object": object,
    "Exception": Exception,
    "ValueError": ValueError,
    "TypeError": TypeError,
    "KeyError": KeyError,
    "IndexError": IndexError,
    "RuntimeError": RuntimeError,
    "StopIteration": StopIteration,
    "AttributeError": AttributeError,
    "NameError": NameError,
}


DANGEROUS_MODULES: Set[str] = {
    "os",
    "sys",
    "subprocess",
    "shutil",
    "socket",
    "http",
    "urllib",
    "requests",
    "pathlib",
    "ctypes",
    "multiprocessing",
    "threading",
    "signal",
    "fcntl",
    "pty",
    "pwd",
    "grp",
    "resource",
    "importlib",
    "builtins",
    "code",
    "codeop",
    "pickle",
    "marshal",
    "shelve",
    "dbm",
    "sqlite3",
    "tempfile",
    "glob",
    "fnmatch",
    "linecache",
    "traceback",
    "inspect",
    "gc",
    "sysconfig",
}


FORBIDDEN_NAMES: Set[str] = {
    "eval",
    "exec",
    "compile",
    "open",
    "input",
    "breakpoint",
    "__import__",
    "exit",
    "quit",
    "help",
    "license",
    "credits",
    "copyright",
}


def safe_import(name: str, *args: Any, **kwargs: Any) -> Any:
    """Block obviously dangerous modules while still allowing pure libraries."""
    root = name.split(".", 1)[0]
    if root in DANGEROUS_MODULES or name.startswith("."):
        raise ImportError(
            f"Import of module '{name}' is not allowed inside the Concierge sandbox."
        )
    return __import__(name, *args, **kwargs)


# ---------------------------------------------------------------------------
# AST safety walker
# ---------------------------------------------------------------------------

class SafetyVisitor(ast.NodeVisitor):
    """Reject constructs that are commonly used to escape restricted environments."""

    def visit_Call(self, node: ast.Call) -> None:
        if isinstance(node.func, ast.Name) and node.func.id in FORBIDDEN_NAMES:
            raise SecurityError(f"Call to '{node.func.id}' is not allowed.")
        if isinstance(node.func, ast.Name) and node.func.id == "getattr":
            if len(node.args) >= 2 and isinstance(node.args[1], ast.Constant):
                attr = str(node.args[1].value)
                if attr.startswith("__") and attr.endswith("__"):
                    raise SecurityError(f"getattr of dunder attribute '{attr}' is not allowed.")
        self.generic_visit(node)

    def visit_Attribute(self, node: ast.Attribute) -> None:
        if node.attr.startswith("__") and node.attr.endswith("__"):
            dangerous = {
                "__class__",
                "__bases__",
                "__subclasses__",
                "__mro__",
                "__globals__",
                "__code__",
                "__builtins__",
                "__dict__",
                "__reduce__",
                "__reduce_ex__",
            }
            if node.attr in dangerous:
                raise SecurityError(f"Access to '{node.attr}' is not allowed.")
        self.generic_visit(node)

    def visit_Name(self, node: ast.Name) -> None:
        if node.id in FORBIDDEN_NAMES:
            raise SecurityError(f"Name '{node.id}' is not allowed.")
        self.generic_visit(node)

    def visit_Global(self, node: ast.Global) -> None:
        raise SecurityError("global statements are not permitted.")

    def visit_Nonlocal(self, node: ast.Nonlocal) -> None:
        raise SecurityError("nonlocal statements are not permitted.")


class SecurityError(Exception):
    pass


# ---------------------------------------------------------------------------
# Sandbox runner
# ---------------------------------------------------------------------------

class Sandbox:
    def __init__(self) -> None:
        self.globals: Dict[str, Any] = {
            "__builtins__": SAFE_BUILTINS,
            "__name__": "__main__",
            "__import__": safe_import,
        }
        SAFE_BUILTINS["__import__"] = safe_import

    def execute(self, code_str: str, filename: str = "<sandbox>") -> str:
        try:
            tree = ast.parse(code_str, filename=filename)
        except SyntaxError as e:
            return f"Sandbox syntax error: {e}"

        try:
            SafetyVisitor().visit(tree)
        except SecurityError as e:
            return f"Sandbox security error: {e}"

        code = compile(tree, filename, "exec")

        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stdout = io.StringIO()
        sys.stderr = io.StringIO()

        try:
            exec(code, self.globals)
            out = sys.stdout.getvalue()
            err = sys.stderr.getvalue()
            if err:
                out = (out + "\n" + err).strip()
            return out if out else "(no output)"
        except Exception:
            return f"Sandbox error:\n{traceback.format_exc()}"
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def main() -> int:
    if len(sys.argv) < 2:
        print("Usage: python sandbox.py <script_to_run>", file=sys.stderr)
        return 1

    script_path = sys.argv[1]
    try:
        with open(script_path, "r", encoding="utf-8") as f:
            user_code = f.read()
    except OSError as e:
        print(f"Cannot read script: {e}", file=sys.stderr)
        return 1

    sandbox = Sandbox()
    result = sandbox.execute(user_code, filename=script_path)
    print(result)
    return 0


if __name__ == "__main__":
    sys.exit(main())
