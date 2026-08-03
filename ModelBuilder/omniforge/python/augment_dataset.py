#!/usr/bin/env python3
import argparse, json, random

def synonym_replace(words):
    # trivial identity – real synonym maps can be plugged in
    return words

def random_deletion(words, p=0.15):
    if len(words) <= 1:
        return words
    return [w for w in words if random.random() > p] or words[:1]

def augment_text(text, methods):
    words = text.split()
    if "synonym" in methods:
        words = synonym_replace(words)
    if "deletion" in methods:
        words = random_deletion(words)
    return " ".join(words)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dataset", required=True)
    ap.add_argument("--augment", default="")
    args = ap.parse_args()
    methods = [m for m in args.augment.split(",") if m]
    with open(args.dataset) as f:
        data = [json.loads(line) for line in f if line.strip()]
    for entry in data:
        if "text" in entry:
            entry["text"] = augment_text(entry["text"], methods)
    out = args.dataset + ".augmented"
    with open(out, "w") as f:
        for e in data:
            f.write(json.dumps(e) + "\n")
    print(f"Augmented dataset saved to {out}")

if __name__ == "__main__":
    main()
