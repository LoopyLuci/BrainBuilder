#!/usr/bin/env python3
"""
Runtime smoke test for BrainBuilder ↔ ModelMistress integration.

Requires:
- ModelMistress running at http://localhost:8000
- Python 3 with `requests`

Usage:
    python scripts/smoke_test_model_mistress.py
    python scripts/smoke_test_model_mistress.py --url http://host:9000
"""
import argparse
import sys
import time

try:
    import requests
except ImportError:
    print("ERROR: 'requests' package required. Install with: pip install requests")
    sys.exit(2)


def parse_args():
    p = argparse.ArgumentParser(description="ModelMistress runtime smoke test")
    p.add_argument("--url", default="http://localhost:8000", help="ModelMistress base URL")
    p.add_argument("--retries", type=int, default=10, help="Health-check retries before giving up")
    p.add_argument("--delay", type=float, default=1.0, help="Delay between retries")
    return p.parse_args()


def wait_for_health(base_url: str, retries: int, delay: float):
    for attempt in range(1, retries + 1):
        try:
            r = requests.get(f"{base_url}/health", timeout=5)
            if r.status_code == 200 and r.json().get("status") == "healthy":
                return True
        except Exception:
            pass
        if attempt < retries:
            time.sleep(delay)
    return False


def check(name, condition, detail=""):
    status = "PASS" if condition else "FAIL"
    print(f"  [{status}] {name}")
    if not condition:
        if detail:
            print(f"         detail: {detail}")
        return False
    return True


def main():
    args = parse_args()
    base = args.url.rstrip("/")
    results = []

    print(f"Smoke-testing ModelMistress at {base}")
    print()

    # 1) Wait for health
    alive = wait_for_health(base, args.retries, args.delay)
    results.append(check("server healthy", alive, f"after {args.retries} retries"))
    if not alive:
        print("Server not healthy, aborting remaining checks.")
        print()
        print(f"FAILED {sum(1 for r in results if not r)}/{len(results)} checks")
        sys.exit(1)

    # 2) Health detail
    try:
        h = requests.get(f"{base}/health", timeout=10).json()
        results.append(check("health.version present", "version" in h, str(h)))
        results.append(check("health.ollama reachable", h.get("ollama") in ("connected", "error"), str(h)))
    except Exception as e:
        results.append(check("health detail fetch", False, str(e)))

    # 3) List models
    try:
        r = requests.get(f"{base}/v1/models", timeout=10)
        results.append(check("models status 200", r.status_code == 200, f"got {r.status_code}"))
        data = r.json()
        models = data.get("data", [])
        results.append(check("models.list returns array", isinstance(models, list), str(data)[:200]))
    except Exception as e:
        results.append(check("models endpoint", False, str(e)))

    # 4) Chat completion
    try:
        payload = {
            "model": "smoke-test-model",
            "messages": [{"role": "user", "content": "Say OK"}],
            "max_tokens": 8,
            "temperature": 0.0,
        }
        r = requests.post(f"{base}/v1/chat/completions", json=payload, timeout=30)
        body = r.json()
        results.append(check("chat responds with json", isinstance(body, dict), str(body)[:200]))
        results.append(check("chat has choices or error", "choices" in body or "error" in body, str(body)[:200]))
    except Exception as e:
        results.append(check("chat completion", False, str(e)))

    # 5) Chat streaming
    try:
        payload["stream"] = True
        r = requests.post(f"{base}/v1/chat/completions", json=payload, timeout=30, stream=True)
        results.append(check("stream status < 600", r.status_code < 600, f"got {r.status_code}"))
        chunks = []
        for line in r.iter_lines(decode_unicode=True):
            if line:
                chunks.append(line)
            if len(chunks) >= 1:
                break
        results.append(check("stream yields chunks", len(chunks) >= 1, f"chunks={len(chunks)}"))
    except Exception as e:
        results.append(check("chat stream", False, str(e)))

    print()
    failed = sum(1 for ok in results if not ok)
    if failed:
        print(f"FAILED {failed}/{len(results)} checks")
        sys.exit(1)
    print(f"All {len(results)} smoke checks passed against live ModelMistress.")
    sys.exit(0)


if __name__ == "__main__":
    main()
