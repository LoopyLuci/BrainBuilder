#!/usr/bin/env python3
import argparse, json, re

POSITIVE = {"good", "great", "love", "excellent", "happy", "wonderful"}
NEGATIVE = {"bad", "hate", "terrible", "awful", "sad", "poor"}

def analyze_sentiment(text: str) -> dict:
    words = set(re.findall(r"[a-z']+", text.lower()))
    pos = len(words & POSITIVE)
    neg = len(words & NEGATIVE)
    score = (pos - neg) / max(pos + neg, 1)
    label = "positive" if score > 0.15 else "negative" if score < -0.15 else "neutral"
    return {"score": round(score, 3), "label": label, "pos": pos, "neg": neg}

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tool", required=True)
    ap.add_argument("--args", default="{}")
    args = ap.parse_args()
    payload = json.loads(args.args)
    if args.tool == "analyze_sentiment":
        print(json.dumps(analyze_sentiment(payload.get("text", ""))))
    else:
        raise SystemExit(f"Unknown tool: {args.tool}")

if __name__ == "__main__":
    main()
