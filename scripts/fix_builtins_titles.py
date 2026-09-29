from pathlib import Path
text = Path("scripts/generate_catalog_models.py").read_text(encoding="utf-8")
old = "chr(39)+chr(92)+chr(39)+chr(39)"
new = "chr(92)+chr(39)"
if old in text:
    text = text.replace(old, new)
    Path("scripts/generate_catalog_models.py").write_text(text, encoding="utf-8")
    print("patched")
else:
    print("not found")
