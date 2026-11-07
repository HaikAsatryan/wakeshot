"""Checks the Pages page before it ships; pages.yml runs it before each deploy.

Fails when: an HTML file does not parse into balanced tags; a local link, image or CSS url() does not resolve inside
the folder; a src or href leaves https://github.com/HaikAsatryan/wakeshot and the page itself; a link into the repo
names a file the repo lacks, or a #heading its Markdown file lacks (README.md for the repo's own address); an image
has no alt; the page has a script, an event handler attribute or an embedded frame; any text file holds an em dash
or an emoji; a colour pair of style.css drops below WCAG 2.2 AA in the light or the dark theme (text 4.5, focus ring
and button edge 3.0); a share tag (og:, twitter:) points outside the page's Pages address or to a missing file, has
an image without alt text, or says a title or description other than the page's own.

Two kinds of folder. site/ itself (it holds build_page.py): the source, every conditional block in place, and each
media/<name> resolved where build_page.py takes it from. A folder build_page.py wrote: what goes live. --final is for
the second: it also fails while a placeholder is left (an HTML comment that starts with "waits for"), while a
conditional block is left (the folder was not built), and on any file no page uses.

Run: python site/check_page.py [--final] [--colours] <folder> (--colours prints every contrast ratio)
Python 3.8 or later, standard library only.
"""

import functools
import html
import re
import sys
import xml.etree.ElementTree as ET
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

# No __pycache__ in site/ when this imports build_page: the folder is the page's source, and git would see it.
sys.dont_write_bytecode = True

REPO = "https://github.com/HaikAsatryan/wakeshot"
REPO_ROOT = Path(__file__).resolve().parent.parent
# Where pages.yml publishes this folder: a project site's address, https://<owner>.github.io/<repository>/. A share
# tag must use the full address, so it names files by it.
PAGES = "https://haikasatryan.github.io/wakeshot/"
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}
URL_ATTRS = {"href", "src", "poster", "action", "formaction", "cite", "data"}
BANNED_TAGS = {"script", "iframe", "frame", "object", "embed", "applet", "base"}
TEXT_SUFFIXES = {".html", ".css", ".svg", ".py", ".txt", ".md", ".xml", ".json"}
# The page must stay plain text: no em dash (the project's writing style), no emoji. The emoji set is the main emoji
# blocks plus the emoji presentation selector, the joiner and the keycap mark; a plain symbol such as (c) is allowed.
# Built from code points, so this file passes its own text check.
EM_DASH = chr(0x2014)
EMOJI_RANGES = [
    (0x1F000, 0x1FAFF), (0x2300, 0x23FF), (0x2600, 0x27BF), (0x2B00, 0x2BFF),
    (0x3030, 0x3030), (0x303D, 0x303D), (0x3297, 0x3297), (0x3299, 0x3299),
    (0xFE0F, 0xFE0F), (0x200D, 0x200D), (0x20E3, 0x20E3),
]
EMOJI = re.compile("[" + "".join(f"{chr(a)}-{chr(b)}" for a, b in EMOJI_RANGES) + "]")
WAITS = re.compile(r"^\s*waits for\b", re.IGNORECASE)
# build_page.py's conditional blocks: "<!-- if released -->" to "<!-- end if -->".
CONDITION = re.compile(r"^\s*(?:if|end if)\b")
# Files no page links: a font's licence travels beside it; the README draws the light lockup from here (the dark one
# is the social card's too); social-card.html is the source of social-card.png and social_card.py draws it;
# build_page.py builds the folder pages.yml publishes.
UNLINKED_OK = {"OFL.txt", "wakeshot-lockup-light.svg", "social-card.html", "social_card.py", "build_page.py"}

# The colour pairs style.css must keep, by custom property name: (front, back, least WCAG ratio). "hero" is the page
# under the hero's two see-through glows, bottom layer first: a pair must hold on the bare page, under each glow alone
# and under both at full strength, because the hero's text sits on every mix between them.
SURFACES = {"hero": ("bg", "glow", "glow-core")}
PAIRS = [
    ("text", "bg", 4.5), ("text", "panel", 4.5), ("text", "hero", 4.5),
    ("muted", "bg", 4.5), ("muted", "panel", 4.5), ("muted", "hero", 4.5),
    ("link", "bg", 4.5), ("link", "panel", 4.5), ("link", "hero", 4.5),
    ("button-text", "button-bg", 4.5), ("on-step", "step", 4.5),
    ("button-bg", "bg", 3.0), ("button-bg", "hero", 3.0),
    ("focus", "bg", 3.0), ("focus", "panel", 3.0), ("focus", "hero", 3.0),
]


class Page(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.stack = []
        self.errors = []
        self.ids = set()
        self.urls = []
        self.waits = []
        self.conditions = []
        self.doctype = False
        self.lang = False
        self.charset = False
        self.title = ""
        self.in_title = False
        self.description = None
        self.share = {}
        self.style = ""
        self.in_style = False

    def where(self):
        return f"line {self.getpos()[0]}"

    def handle_decl(self, decl):
        if decl.lower() == "doctype html":
            self.doctype = True

    def handle_starttag(self, tag, attrs):
        self.check_tag(tag, attrs)
        if tag not in VOID:
            self.stack.append((tag, self.where()))
        if tag == "title":
            self.in_title = True
        if tag == "style":
            self.in_style = True

    def handle_startendtag(self, tag, attrs):
        self.check_tag(tag, attrs)

    def check_tag(self, tag, attrs):
        if tag in BANNED_TAGS:
            self.errors.append(f"{self.where()}: <{tag}> is not allowed (no scripts, frames or embeds)")
        seen = set()
        for name, value in attrs:
            if name in seen:
                self.errors.append(f"{self.where()}: <{tag}> repeats the attribute {name}")
            seen.add(name)
            if name.startswith("on"):
                self.errors.append(f"{self.where()}: <{tag} {name}> is a script")
            if name == "id":
                if value in self.ids:
                    self.errors.append(f"{self.where()}: id {value!r} is used twice")
                self.ids.add(value)
            if name in URL_ATTRS and value is not None:
                self.urls.append((self.where(), tag, value))
            if name == "srcset" and value:
                for part in value.split(","):
                    if part.strip():
                        self.urls.append((self.where(), tag, part.split()[0]))
            if name == "style" and value:
                for url in css_urls(value):
                    self.urls.append((self.where(), tag, url))
        names = dict(attrs)
        if tag == "img" and "alt" not in names:
            self.errors.append(f"{self.where()}: <img> has no alt (alt=\"\" for a picture that only decorates)")
        if tag == "html" and names.get("lang"):
            self.lang = True
        if tag == "meta" and (names.get("charset") or "").lower() == "utf-8":
            self.charset = True
        if tag == "meta" and (names.get("http-equiv") or "").lower() == "refresh":
            self.errors.append(f"{self.where()}: a meta refresh is not allowed")
        if tag == "meta":
            key = names.get("property") or names.get("name") or ""
            if key == "description":
                self.description = names.get("content")
            if key.startswith(("og:", "twitter:")):
                self.share[key] = names.get("content") or ""

    def handle_endtag(self, tag):
        if tag in VOID:
            self.errors.append(f"{self.where()}: </{tag}> closes a void element")
            return
        if not self.stack:
            self.errors.append(f"{self.where()}: </{tag}> has no open tag")
            return
        open_tag, opened = self.stack.pop()
        if open_tag != tag:
            self.errors.append(f"{self.where()}: </{tag}> closes <{open_tag}> opened at {opened}")
        if tag == "title":
            self.in_title = False
        if tag == "style":
            self.in_style = False

    def handle_data(self, data):
        if self.in_title:
            self.title += data
        if self.in_style:
            self.style += data

    def handle_comment(self, data):
        if WAITS.match(data):
            self.waits.append(f"{self.where()}: {data.strip()[:100]}")
        if CONDITION.match(data):
            self.conditions.append(f"{self.where()}: <!--{data}-->")


def css_urls(text):
    found = re.findall(r"""url\(\s*(['"]?)(.*?)\1\s*\)""", text)
    found = [u for _, u in found]
    found += re.findall(r"""@import\s+['"]([^'"]+)['"]""", text)
    return found


@functools.lru_cache(maxsize=None)
def heading_ids(md_path):
    """The ids GitHub gives the headings of a Markdown file: lower case, punctuation dropped, spaces to hyphens, a
    repeat gets -1, -2 and so on."""
    ids, seen, fence = set(), {}, False
    for line in md_path.read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith(("```", "~~~")):
            fence = not fence
            continue
        m = None if fence else re.match(r"#{1,6}\s+(.+?)\s*#*\s*$", line)
        if not m:
            continue
        text = re.sub(r"<[^>]+>", "", m.group(1))
        text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
        slug = re.sub(r"[^\w\- ]", "", text.strip().lower()).replace(" ", "-")
        count = seen.get(slug, 0)
        seen[slug] = count + 1
        ids.add(slug if count == 0 else f"{slug}-{count}")
    return frozenset(ids)


def check_repo_url(url):
    """An error text for a link into the repo that the repo does not have, else None. Only main's files and their
    headings are checked, against this checkout; releases, issues and the like pass."""
    parts = urlsplit(url)
    path = parts.path[len(urlsplit(REPO).path):]
    fragment = unquote(parts.fragment)
    if path in ("", "/"):
        if fragment and fragment != "readme" and fragment not in heading_ids(REPO_ROOT / "README.md"):
            return f"{url}: README.md has no heading #{fragment}"
        return None
    m = re.fullmatch(r"/(blob|tree)/main/(.+?)/?", path)
    if not m:
        return None
    target = REPO_ROOT / unquote(m.group(2))
    if not (target.is_file() if m.group(1) == "blob" else target.is_dir()):
        return f"{url}: {m.group(2)} is not in the repo"
    if fragment and target.suffix == ".md" and fragment not in heading_ids(target):
        return f"{url}: {m.group(2)} has no heading #{fragment}"
    return None


def check_url(root, base_file, url, ids, media=None):
    """Returns an error text, or None when the URL is allowed and resolves. media, for the source folder: finds the
    file build_page.py takes media/<name> from."""
    url = url.strip()
    if not url:
        return "empty URL"
    if url.startswith("//"):
        return f"{url}: protocol-relative URL to another host"
    parts = urlsplit(url)
    if parts.scheme:
        # The repo itself or a path under it; a look-alike such as .../wakeshot-evil does not pass.
        if url == REPO or any(url.startswith(REPO + c) for c in "/#?"):
            return check_repo_url(url)
        return f"{url}: outside {REPO} and the page"
    if not parts.path:
        if parts.fragment and parts.fragment not in ids:
            return f"{url}: no element with this id"
        return None
    target = (base_file.parent / unquote(parts.path)).resolve()
    try:
        rel = target.relative_to(root).as_posix()
    except ValueError:
        return f"{url}: leaves the page folder"
    if target.is_dir():
        target = target / "index.html"
    if not target.is_file():
        if media and rel.startswith("media/") and media(rel[len("media/"):]):
            return None
        return f"{url}: file not found"
    return None


def check_share(root, page):
    """Errors for the page's share tags, and the files they name (which count as linked)."""
    errors, used = [], set()
    tags = page.share
    for key in ("og:image", "twitter:image"):
        if key not in tags:
            continue
        url = tags[key]
        if not url.startswith(PAGES):
            errors.append(f"{key} {url}: not under {PAGES}")
        else:
            target = (root / unquote(urlsplit(url[len(PAGES):]).path)).resolve()
            if not target.is_file():
                errors.append(f"{key} {url}: file not found in the folder")
            else:
                used.add(target)
        if not tags.get(key + ":alt", "").strip():
            errors.append(f"{key} has no {key}:alt")
    if "og:url" in tags and tags["og:url"] != PAGES:
        errors.append(f"og:url {tags['og:url']}: not {PAGES}")
    if "og:title" in tags and tags["og:title"] != page.title.strip():
        errors.append("og:title differs from <title>")
    if "og:description" in tags and tags["og:description"] != page.description:
        errors.append("og:description differs from the description meta tag")
    return errors, used


def check_text(path, text):
    errors = []
    plain = html.unescape(text) if path.suffix in (".html", ".svg") else text
    for number, line in enumerate(plain.splitlines(), 1):
        if EM_DASH in line:
            errors.append(f"{path.name} line {number}: em dash")
        for mark in EMOJI.findall(line):
            errors.append(f"{path.name} line {number}: emoji U+{ord(mark):04X}")
    return errors


def parse_colour(value):
    """Returns (r, g, b, alpha) with r, g, b in 0..1, or None for a value that is not a plain colour."""
    value = value.strip().lower()
    m = re.fullmatch(r"#([0-9a-f]{3}|[0-9a-f]{6})", value)
    if m:
        h = m.group(1)
        if len(h) == 3:
            h = "".join(c * 2 for c in h)
        return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)) + (1.0,)
    m = re.fullmatch(r"rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)", value)
    if m:
        alpha = float(m.group(4)) if m.group(4) is not None else 1.0
        return tuple(int(m.group(i)) / 255 for i in (1, 2, 3)) + (alpha,)
    return None


def over(top, below):
    """A see-through colour laid over an opaque one."""
    a = top[3]
    return tuple(top[i] * a + below[i] * (1 - a) for i in range(3)) + (1.0,)


def luminance(rgb):
    lin = [c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in rgb[:3]]
    return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]


def ratio(a, b):
    hi, lo = sorted((luminance(a), luminance(b)), reverse=True)
    return (hi + 0.05) / (lo + 0.05)


def custom_properties(block):
    return dict(re.findall(r"--([\w-]+)\s*:\s*([^;]+);", block))


def themes(css):
    """The custom properties of the light and the dark theme. Dark starts from light: a property the dark block does
    not set keeps its light value, as in the browser."""
    css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
    light = re.search(r"(?<![\w-]):root\s*\{([^}]*)\}", css)
    dark = re.search(r"@media\s*\(prefers-color-scheme:\s*dark\)\s*\{\s*:root\s*\{([^}]*)\}", css)
    base = custom_properties(light.group(1)) if light else {}
    return {"light": base, "dark": {**base, **(custom_properties(dark.group(1)) if dark else {})}}


def surfaces(props, name):
    """Every opaque colour a back surface can show: the bare one, or for a layered surface each mix of its glows."""
    layers = SURFACES.get(name, (name,))
    base = parse_colour(props[layers[0]])
    if base is None or base[3] != 1.0:
        raise ValueError(f"--{layers[0]} must be an opaque colour")
    found = []
    for mask in range(1 << (len(layers) - 1)):
        colour = base
        for i, layer in enumerate(layers[1:]):
            if mask >> i & 1:
                top = parse_colour(props[layer])
                if top is None:
                    raise ValueError(f"--{layer} is not a plain colour")
                colour = over(top, colour)
        found.append(colour)
    return found


def check_colours(rel, css):
    """Errors for each pair of PAIRS below its least ratio, in either theme, and the table of all ratios."""
    errors, table = [], []
    for theme, props in themes(css).items():
        for front, back, least in PAIRS:
            names = [front] + list(SURFACES.get(back, (back,)))
            missing = [n for n in names if n not in props]
            if missing:
                errors.append(f"{rel}: {theme} theme has no --{', --'.join(missing)}")
                continue
            try:
                fg = parse_colour(props[front])
                if fg is None or fg[3] != 1.0:
                    raise ValueError(f"--{front} must be an opaque colour")
                worst = min(ratio(fg, bg) for bg in surfaces(props, back))
            except ValueError as e:
                errors.append(f"{rel}: {theme} theme: {e}")
                continue
            table.append(f"{theme:5} {worst:5.2f} (>= {least}) --{front} on {back}")
            if worst < least:
                errors.append(f"{rel}: {theme} theme: --{front} on {back} is {worst:.2f}, below {least}")
    return errors, table


def check_svg(root, path, errors):
    try:
        tree = ET.parse(path)
    except ET.ParseError as e:
        errors.append(f"{path.name}: does not parse as XML: {e}")
        return
    for node in tree.iter():
        tag = node.tag.split("}")[-1]
        if tag in ("script", "foreignObject"):
            errors.append(f"{path.name}: <{tag}> is not allowed")
        for name, value in node.attrib.items():
            local = name.split("}")[-1]
            if local.startswith("on"):
                errors.append(f"{path.name}: <{tag} {local}> is a script")
            if local in ("href", "src"):
                problem = check_url(root, path, value, set())
                if problem:
                    errors.append(f"{path.name}: {problem}")
        if tag == "style" and node.text:
            for url in css_urls(node.text):
                problem = check_url(root, path, url, set())
                if problem:
                    errors.append(f"{path.name}: {problem}")


def main(argv):
    final = "--final" in argv
    show_colours = "--colours" in argv
    rest = [a for a in argv if a not in ("--final", "--colours")]
    if len(rest) != 1:
        print(__doc__)
        return 2
    root = Path(rest[0]).resolve()
    if not (root / "index.html").is_file():
        print(f"{root}: no index.html")
        return 2

    media = None
    if (root / "build_page.py").is_file():
        # The build script next to this one: it knows where each media file comes from in this checkout.
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import build_page

        media = build_page.media_file

    errors, waits, conditions, used, ratios = [], [], [], set(), []
    files = sorted(p for p in root.rglob("*") if p.is_file() and "__pycache__" not in p.parts)
    for path in files:
        rel = path.relative_to(root).as_posix()
        if path.suffix in TEXT_SUFFIXES:
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                errors.append(f"{rel}: not UTF-8")
                continue
            errors += check_text(path, text)
        if path.suffix == ".html":
            page = Page()
            page.feed(text)
            page.close()
            errors += [f"{rel} {e}" for e in page.errors]
            errors += [f"{rel}: <{tag}> opened at {at} is never closed" for tag, at in page.stack]
            if not page.doctype:
                errors.append(f"{rel}: no <!DOCTYPE html>")
            if not page.lang:
                errors.append(f"{rel}: <html> has no lang")
            if not page.charset:
                errors.append(f"{rel}: no <meta charset=\"utf-8\">")
            if not page.title.strip():
                errors.append(f"{rel}: no <title>")
            for at, tag, url in page.urls:
                problem = check_url(root, path, url, page.ids, media)
                if problem:
                    errors.append(f"{rel} {at}: <{tag}> {problem}")
                elif not urlsplit(url).scheme and urlsplit(url).path:
                    used.add((path.parent / unquote(urlsplit(url).path)).resolve())
            for url in css_urls(page.style):
                problem = check_url(root, path, url, set())
                if problem:
                    errors.append(f"{rel}: <style> url() {problem}")
                elif not urlsplit(url).scheme:
                    used.add((path.parent / unquote(urlsplit(url).path)).resolve())
            share_errors, share_used = check_share(root, page)
            errors += [f"{rel}: {e}" for e in share_errors]
            used |= share_used
            waits += [f"{rel} {w}" for w in page.waits]
            conditions += [f"{rel} {c}" for c in page.conditions]
        if path.suffix == ".css":
            for url in css_urls(text):
                problem = check_url(root, path, url, set())
                if problem:
                    errors.append(f"{rel}: url() {problem}")
                elif not urlsplit(url).scheme:
                    used.add((path.parent / unquote(urlsplit(url).path)).resolve())
            if "prefers-color-scheme: dark" not in text:
                errors.append(f"{rel}: no prefers-color-scheme: dark block")
            if ":root" in text:
                colour_errors, table = check_colours(rel, text)
                errors += colour_errors
                ratios += [f"{rel} {row}" for row in table]
        if path.suffix == ".svg":
            check_svg(root, path, errors)

    unused = [p.relative_to(root).as_posix() for p in files
              if p not in used and p.name not in ("index.html", Path(__file__).name) and p.name not in UNLINKED_OK]
    if final:
        errors += [f"still waits: {w}" for w in waits]
        errors += [f"not built (run build_page.py): {c}" for c in conditions]
        errors += [f"{u} is not used by the page: leave it out of the build" for u in unused]

    print(f"checked {len(files)} files in {root}")
    for row in ratios if show_colours else []:
        print(f"contrast: {row}")
    for w in waits if not final else []:
        print(f"waits: {w}")
    for u in unused if not final else []:
        print(f"note: {u} is not linked from any page")
    for e in errors:
        print(f"error: {e}")
    print(f"{len(errors)} errors, {len(waits)} spots wait for other cards")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
