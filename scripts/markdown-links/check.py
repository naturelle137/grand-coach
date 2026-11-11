#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Offline checks for the Markdown subset documented in docs/testing.md."""

import argparse
import html
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit


def blank(text):
    """Keep offsets/line numbers when hiding examples and comments."""
    return re.sub(r"[^\n]", " ", text)


def prose(text):
    text = re.sub(r"<!--.*?(?:-->|\Z)", lambda m: blank(m[0]), text, flags=re.S)
    lines = text.splitlines(keepends=True)
    fence = None
    for i, line in enumerate(lines):
        if fence:
            if re.match(r"^ {0,3}" + re.escape(fence[0]) +
                        r"{" + str(len(fence)) + r",}\s*$", line):
                fence = None
            lines[i] = blank(line)
        else:
            match = re.match(r"^ {0,3}(`{3,}|~{3,})", line)
            if match:
                fence = match[1]
                lines[i] = blank(line)
            elif line.startswith(("    ", "\t")):
                lines[i] = blank(line)
    return "".join(lines)


def without_code(text):
    return re.sub(r"(`+)(?!`)(.*?)(?<!`)\1(?!`)",
                  lambda m: blank(m[0]), text, flags=re.S)


def unescape(text):
    return html.unescape(re.sub(r"\\([!\"#$%&'()*+,\-./:;<=>?@\[\]^_`{|}~\\])", r"\1", text))


def label_key(text):
    return " ".join(unescape(text).split()).casefold()


def destination(text, start):
    """Read angle-bracket or balanced bare destination; return value/end."""
    i = start
    while i < len(text) and text[i].isspace():
        i += 1
    if i < len(text) and text[i] == "<":
        end = re.search(r"(?<!\\)>", text[i + 1:])
        if end:
            stop = i + 1 + end.start()
            return text[i + 1:stop], stop + 1
        return None
    begin, depth = i, 0
    while i < len(text):
        char = text[i]
        if char == "\\" and i + 1 < len(text):
            i += 2
            continue
        if char.isspace() or (char == ")" and depth == 0):
            break
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        i += 1
    if depth:
        return None
    return text[begin:i], i


def links(text):
    """Yield (offset, target) including definitions, images and references."""
    text = without_code(prose(text))
    definitions = {}
    definition_re = r"^ {0,3}\[((?:\\.|[^\]\\\n])+)\]:[ \t]*"
    for match in re.finditer(definition_re, text, re.M):
        parsed = destination(text, match.end())
        if parsed:
            target, end = parsed
            definitions.setdefault(label_key(match[1]), target)
            yield match.start(), target
            text = text[:match.start()] + blank(text[match.start():end]) + text[end:]
    # Nested labels are allowed (e.g. linked images). Escaped brackets are text.
    stack = []
    i = 0
    while i < len(text):
        if text[i] == "\\":
            i += 2
            continue
        if text[i] == "[":
            stack.append(i)
        elif text[i] == "]" and stack:
            start = stack.pop()
            label = text[start + 1:i]
            end = i + 1
            if end < len(text) and text[end] == "(":
                parsed = destination(text, end + 1)
                if parsed:
                    target, stop = parsed
                    tail = re.match(r'''\s*(?:"[^"\n]*"|'[^'\n]*'|\([^\n]*?\))?\s*\)''', text[stop:])
                    if tail:
                        yield start, target
                        i = stop + tail.end() - 1
            elif end < len(text) and text[end] == "[":
                ref = re.match(r"\[((?:\\.|[^\]\\])*)\]", text[end:])
                if ref:
                    key = label_key(ref[1] or label)
                    if key in definitions:
                        yield start, definitions[key]
                    i = end + ref.end() - 1
            elif label_key(label) in definitions:
                yield start, definitions[label_key(label)]
        i += 1


def anchors(text):
    lines = prose(text).splitlines()
    used = set()
    for i, line in enumerate(lines):
        atx = re.match(r"^ {0,3}#{1,6}(?:\s+|$)(.*)", line)
        if atx:
            title = re.sub(r"\s+#+\s*$", "", atx[1]).strip()
        elif i + 1 < len(lines) and line.strip() and re.fullmatch(r" {0,3}(?:=+|-+)\s*", lines[i + 1]):
            title = line.strip()
        else:
            continue
        title = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", title)
        title = re.sub(r"<[^>]*>", "", title)
        title = re.sub(r"(?<!\w)_{1,3}(.+?)_{1,3}(?!\w)", r"\1", title)
        title = unescape(title).lower()
        # GitHub-style punctuation removal; preserve Unicode letters, digits,
        # underscores and hyphens. Formatting backticks/emphasis disappear.
        title = re.sub(r"[^\w\-\s]", "", title).replace(" ", "-")
        slug, suffix = title, 0
        while slug in used:
            suffix += 1
            slug = f"{title}-{suffix}"
        used.add(slug)
    return used


def check(root):
    raw = subprocess.check_output(["git", "ls-files", "-z"], cwd=root)
    tracked = {p.decode("utf-8") for p in raw.split(b"\0") if p}
    sources = sorted(p for p in tracked if p.lower().endswith(".md"))
    cache = {}
    errors = []
    count = 0
    for source in sources:
        path = root / source
        if not path.is_file():
            errors.append(f"{source}:1: missing tracked Markdown source")
            continue
        text = path.read_text(encoding="utf-8")
        for offset, raw_target in links(text):
            target = unescape(raw_target)
            parts = urlsplit(target)
            if parts.scheme or parts.netloc or target.startswith("//"):
                continue
            count += 1
            target_path = unquote(parts.path)
            resolved = ((root if target_path.startswith("/") else path.parent) /
                        target_path.lstrip("/")).resolve() if target_path else path.resolve()
            reason = None
            try:
                relative = resolved.relative_to(root).as_posix()
            except ValueError:
                relative = ""
                reason = "target escapes repository"
            if reason is None and relative != "." and relative not in tracked and not any(
                    name.startswith(relative.rstrip("/") + "/") for name in tracked):
                reason = "missing tracked target"
            elif reason is None and not resolved.exists():
                reason = "missing target"
            fragment = unquote(parts.fragment)
            if reason is None and fragment and resolved.suffix.lower() == ".md":
                if relative not in cache:
                    cache[relative] = anchors(resolved.read_text(encoding="utf-8"))
                if fragment not in cache[relative]:
                    reason = f"missing heading anchor #{fragment}"
            if reason:
                line = text.count("\n", 0, offset) + 1
                column = offset - text.rfind("\n", 0, offset)
                errors.append(f"{source}:{line}:{column}: {raw_target!r}: {reason}")
    for error in errors:
        print(error, file=sys.stderr)
    print(f"Markdown links: {len(sources)} tracked documents, {count} local links, {len(errors)} errors")
    return bool(errors)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, help="Git repository (default: current repository)")
    args = parser.parse_args()
    root = args.root or Path(subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True).strip())
    try:
        return check(root.resolve())
    except (OSError, UnicodeError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Markdown links: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
