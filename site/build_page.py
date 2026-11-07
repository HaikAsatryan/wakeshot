"""Builds the folder pages.yml publishes: the page as it should look right now, with the media files it shows.

- Copies site/ without the repo's own files: the scripts, the social card's sources, the lockups only the README and
  the social card use, and the font only the social card loads.
- Copies each media file the page names (media/<name>) from media/out/ or media/public/shots/, so the repo never
  holds a second copy that goes stale when the video is rendered again.
- Keeps or drops each conditional block of the page by what is out on GitHub at build time. A block is a line
  "<!-- if released -->" (or "if not released", "if winget", "if not winget"), the lines it holds, and a line
  "<!-- end if -->"; blocks do not nest. "released": the latest stable release has every Setup file the page links,
  so the download links work. "winget": winget-pkgs has the package, so "winget install" works. GitHub is asked
  unless --released and --winget give the answer; any answer but a clear yes or no stops the build, so a passing
  hiccup never publishes a page that says the wrong thing.

Run: python site/build_page.py [--released yes|no] [--winget yes|no] <new or empty output folder>
GH_TOKEN, when set, signs the GitHub calls (pages.yml sets it; unsigned, GitHub allows 60 calls an hour).
Python 3.8 or later, standard library only.
"""

import json
import os
import re
import shutil
import sys
import urllib.error
import urllib.request
from pathlib import Path

# No __pycache__ in site/: the folder is the page's source, and git would see it.
sys.dont_write_bytecode = True
import check_page  # noqa: E402

SITE = Path(__file__).resolve().parent
REPO_ROOT = SITE.parent
OWNER_REPO = "HaikAsatryan/wakeshot"
WINGET_ID = "HaikAsatryan.Wakeshot"
# Where the page's media files are made (media/README.md); the page names them as media/<file name>.
MEDIA_SOURCES = ("media/out", "media/public/shots")
# Files of site/ that serve the repo, not the page. The check (check_page.py --final) fails on any file the page
# does not use, so a new repo-only file shows up there.
REPO_ONLY = {
    "build_page.py",
    "check_page.py",
    "social_card.py",
    "social-card.html",
    "wakeshot-lockup-light.svg",
    "wakeshot-lockup-dark.svg",
    "fonts/InstrumentSans-Regular.woff2",
}
FACTS = ("released", "winget")
BLOCK = re.compile(
    r"^[ \t]*<!--[ \t]*if[ \t]+(not[ \t]+)?([\w-]+)[ \t]*-->[ \t]*\n(.*?)^[ \t]*<!--[ \t]*end if[ \t]*-->[ \t]*\n",
    re.M | re.S,
)
MARK = re.compile(r"<!--\s*(?:if|end if)\b")
DOWNLOAD = re.compile(re.escape(f"https://github.com/{OWNER_REPO}/releases/latest/download/") + r"([^\"'\s#?]+)")


class BuildError(Exception):
    pass


def media_file(name):
    """The one source file for media/<name>, or None when no source folder (or more than one) has it."""
    found = [REPO_ROOT / folder / name for folder in MEDIA_SOURCES if (REPO_ROOT / folder / name).is_file()]
    return found[0] if len(found) == 1 else None


def apply_blocks(text, facts):
    """The page with each block kept or dropped by facts; BuildError on an unknown fact, a nested or unclosed block,
    or a marker that is not on a line of its own."""

    def keep(m):
        negate, name, body = m.group(1), m.group(2), m.group(3)
        if name not in facts:
            raise BuildError(f'unknown condition "{name}" (known: {", ".join(sorted(facts))})')
        if MARK.search(body):
            raise BuildError(f'the block "if {negate or ""}{name}" holds another marker: blocks do not nest')
        return body if facts[name] != bool(negate) else ""

    out = BLOCK.sub(keep, text)
    left = MARK.search(out)
    if left:
        line = out.count("\n", 0, left.start()) + 1
        raise BuildError(f"line {line} of the built page: a marker without its pair, or not on a line of its own")
    return out


def ask_github(path):
    """The JSON GitHub returns for path, or None for a 404. BuildError for anything else."""
    api = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    request = urllib.request.Request(
        api + path,
        headers={
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "wakeshot-pages-build",
        },
    )
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return None
        raise BuildError(f"GitHub gave HTTP {e.code} for {path}") from e
    except (urllib.error.URLError, OSError, ValueError) as e:
        raise BuildError(f"GitHub did not answer for {path}: {e}") from e


def is_released(downloads):
    """True when the latest stable release has every Setup file the page links. BuildError when a stable release
    exists but lacks one: the page would offer a broken download."""
    latest = ask_github(f"/repos/{OWNER_REPO}/releases/latest")
    if latest is None:
        return False
    names = {asset.get("name") for asset in latest.get("assets", [])}
    missing = sorted(set(downloads) - names)
    if missing:
        raise BuildError(f"release {latest.get('tag_name')} lacks {', '.join(missing)}, which the page links")
    return True


def is_on_winget():
    # winget-pkgs keeps each package under manifests/<first letter>/<publisher>/<name>/ once its first PR is merged.
    publisher, name = WINGET_ID.split(".", 1)
    path = f"/repos/microsoft/winget-pkgs/contents/manifests/{publisher[0].lower()}/{publisher}/{name}"
    return ask_github(path) is not None


def build(out, facts):
    """Writes the page into out (new or empty) and returns the list of files written."""
    if out.exists() and any(out.iterdir()):
        raise BuildError(f"{out} is not empty")
    written = []
    for path in sorted(p for p in SITE.rglob("*") if p.is_file()):
        rel = path.relative_to(SITE).as_posix()
        if rel in REPO_ONLY or "__pycache__" in path.parts:
            continue
        target = out / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        if path.suffix == ".html":
            text = apply_blocks(path.read_text(encoding="utf-8"), facts)
            target.write_text(text, encoding="utf-8", newline="\n")
            page = check_page.Page()
            page.feed(text)
            page.close()
            for _, _, url in page.urls:
                if url.startswith("media/"):
                    name = url[len("media/"):].split("#")[0].split("?")[0]
                    source = media_file(name)
                    if source is None:
                        raise BuildError(f"{rel} names media/{name}: not in exactly one of {', '.join(MEDIA_SOURCES)}")
                    (out / "media").mkdir(exist_ok=True)
                    if not (out / "media" / name).exists():
                        shutil.copyfile(source, out / "media" / name)
                        written.append(f"media/{name}")
        else:
            shutil.copyfile(path, target)
        written.append(rel)
    return sorted(written)


def main(argv):
    given, rest = {}, []
    args = iter(argv)
    for arg in args:
        key = arg[2:] if arg.startswith("--") else None
        if key in FACTS:
            value = next(args, "")
            if value not in ("yes", "no"):
                print(f"--{key} takes yes or no")
                return 2
            given[key] = value == "yes"
        else:
            rest.append(arg)
    if len(rest) != 1 or rest[0].startswith("--"):
        print(__doc__)
        return 2
    out = Path(rest[0]).resolve()
    try:
        facts = dict(given)
        if "released" not in facts:
            page = (SITE / "index.html").read_text(encoding="utf-8")
            facts["released"] = is_released(set(DOWNLOAD.findall(page)))
        if "winget" not in facts:
            facts["winget"] = is_on_winget()
        print("build: " + ", ".join(f"{k} {'yes' if facts[k] else 'no'}" for k in FACTS))
        written = build(out, facts)
    except BuildError as e:
        print(f"build: error: {e}")
        return 1
    print(f"build: {len(written)} files in {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
