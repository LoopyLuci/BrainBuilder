from pathlib import Path
text = Path("Z:/Projects/BrainBuilder/scripts/generate_catalog_models.py").read_text(encoding="utf-8")
old = """    title = m["name"].replace("'", "\\\\'")"""
new = """    title = m["name"].replace("'", "\\'")"""
if old in text:
    text = text.replace(old, new)
    Path("Z:/Projects/BrainBuilder/scripts/generate_catalog_models.py").write_text(text, encoding="utf-8")
    print("patched escape")
else:
    print("pattern not found")
    for i, line in enumerate(text.splitlines(), 1):
        if "replace(\"'\"" in line:
            print(i, repr(line))
