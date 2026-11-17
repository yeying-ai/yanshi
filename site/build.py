#!/usr/bin/env python3
"""Assemble the GitHub Pages site into site/book/.

Layout produced under site/book/:
  index.html, styles.css, app.js, assets/  -> Product landing page (/yanshi/)
  docs/                                    -> mdBook manual (/yanshi/docs/)
"""

from pathlib import Path
import shutil
import subprocess
import sys

SITE_DIR = Path(__file__).resolve().parent
LANDING_DIR = SITE_DIR / "landing"
OUT_DIR = SITE_DIR / "book"
DOCS_OUT_DIR = OUT_DIR / "docs"

REQUIRED_OUTPUTS = (
  OUT_DIR / "index.html",
  OUT_DIR / "styles.css",
  OUT_DIR / "app.js",
  OUT_DIR / "assets" / "favicon.svg",
  DOCS_OUT_DIR / "index.html",
  DOCS_OUT_DIR / "STUDIO_CATALOGUE.html",
)


def build_site() -> None:
  if not LANDING_DIR.is_dir():
    print(f"site/build.py: missing landing directory: {LANDING_DIR}", file=sys.stderr)
    sys.exit(1)

  if OUT_DIR.exists():
    shutil.rmtree(OUT_DIR)
  OUT_DIR.mkdir(parents=True, exist_ok=True)

  shutil.copytree(LANDING_DIR, OUT_DIR, dirs_exist_ok=True)
  subprocess.run(["mdbook", "build", str(SITE_DIR)], check=True)
  (OUT_DIR / ".nojekyll").write_text("", encoding="utf-8")

  missing = [str(path.relative_to(SITE_DIR)) for path in REQUIRED_OUTPUTS if not path.is_file()]
  if missing:
    print("site/build.py: missing expected build artifacts:", file=sys.stderr)
    for item in missing:
      print(f"  {item}", file=sys.stderr)
    sys.exit(1)

  print(f"Built landing page -> {OUT_DIR / 'index.html'}")
  print(f"Built mdBook manual -> {DOCS_OUT_DIR / 'index.html'}")


if __name__ == "__main__":
  build_site()
