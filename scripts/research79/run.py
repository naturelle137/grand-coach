#!/usr/bin/env python3
# SPDX-License-Identifier: EUPL-1.2
"""Disposable #79 harness: frozen corpus through templates and a local llama.cpp model.

Needs retained development inputs outside git (see docs/coach-language-research.md).
Usage: run.py --runtime DIR --model FILE --out RESULT.json [--only N]
"""
import argparse
import json
import os
import re
import statistics
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpus as cp  # noqa: E402

SUIT = {k: v for k, v in cp.SUITS.items()}
TEXT = {  # the template arm: Fluent-style rendering of validated (id, param)
    "en": {"count_exact": "Exactly {p} trumps are still out.",
           "count_estimated": "About {p} trumps are probably still out.",
           "void_proven": "Proven: nobody can follow {p} any more.",
           "insufficient_evidence": "The evidence does not settle this yet."},
    "de": {"count_exact": "Es sind genau {p} Trümpfe draußen.",
           "count_estimated": "Wahrscheinlich sind noch etwa {p} Trümpfe draußen.",
           "void_proven": "Bewiesen: {p} kann niemand mehr bedienen.",
           "insufficient_evidence": "Die Lage reicht dafür noch nicht aus."},
}
GRAMMAR = ('root ::= id "|" param\n'
           'id ::= "count_exact" | "count_estimated" | "void_proven" | "insufficient_evidence"\n'
           'param ::= [0-9] [0-9]? | "kreuz" | "pik" | "herz" | "karo" | "none"\n')


def render(lang, i, p):
    if i == "void_proven":
        p = SUIT[p][0 if lang == "de" else 1]
    return TEXT[lang][i].format(p=p)


def prompt(c, grammar):
    en = c["lang"] == "en"
    if c["sources"] is not None:
        ev = "sources disagree: " + " vs ".join(map(str, c["sources"]))
    else:
        ev = f"value={c['value'] if c['value'] is not None else 'missing'}"
    lines = [("Pick one phrase id and its parameter from the evidence. Use only the "
              "evidence. Answer exactly as ID|PARAM (PARAM is a number, a suit in "
              "lower case, or none).") if en else
             ("Wähle aus den Belegen eine Phrasen-ID und ihren Parameter. Nutze nur die "
              "Belege. Antworte genau als ID|PARAM (PARAM ist eine Zahl, eine Farbe "
              "klein geschrieben oder none)."),
             f"concept={c['concept']} certainty={c['certainty']} {ev}",
             "allowed=" + ",".join(c["allowed"])]
    if c["note"]:
        lines.append("note: " + c["note"])
    user = "\n".join(lines)
    if grammar:
        return (f"<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"
                "<think>\n\n</think>\n\n")
    return f"<|im_start|>user\n{user} /no_think<|im_end|>\n<|im_start|>assistant\n"


def infer(args, c, grammar, threads=4):
    cmd = [os.path.join(args.runtime, "llama-completion"), "-m", args.model, "-p",
           prompt(c, grammar), "-n", "24", "-no-cnv", "--temp", "0", "--seed", "1",
           "-t", str(threads), "-c", "512", "--no-display-prompt"]
    if grammar:
        cmd += ["--grammar", GRAMMAR]
    env = dict(os.environ, LD_LIBRARY_PATH=args.runtime)
    t0 = time.monotonic()
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env)
    out = p.stdout.read()
    _, status, ru = os.wait4(p.pid, 0)
    return (out.decode("utf-8", "replace"), time.monotonic() - t0, ru.ru_maxrss, status)


def strict(c, text):
    """Whole output must be exactly ID|PARAM with an allowed id."""
    m = re.fullmatch(r"\s*([a-z_]+)\|([a-z0-9]+)\s*(?:\[end of text\])?\s*", text)
    return [m.group(1), m.group(2)] if m and m.group(1) in c["allowed"] else None


def lenient(c, text):
    body = re.sub(r"<think>.*?</think>", "", text, flags=re.S)
    m = re.search(r"([a-z_]+)\|([a-z0-9]+)", body)
    return [m.group(1), m.group(2)] if m and m.group(1) in c["allowed"] else None


def validate(c, parsed):
    """Post-check against the supplied evidence; True means the output may be shown."""
    return parsed is not None and parsed == c["expected"]


def select(c, output):
    """Reject ungrounded model output and render the deterministic fallback."""
    parsed = strict(c, output)
    accepted = validate(c, parsed)
    pair = parsed if accepted else cp.reference(c)
    return {"accepted": accepted, "pair": pair, "text": render(c["lang"], *pair)}


def pct(xs, q):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, round(q * (len(xs) - 1)))]


def evict(path):
    fd = os.open(path, os.O_RDONLY)
    os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
    os.close(fd)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runtime", required=True)
    ap.add_argument("--model", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--only", type=int)
    args = ap.parse_args()
    if args.only is not None and args.only <= 0:
        ap.error("--only must be positive")
    if os.path.exists(args.out):
        ap.error("--out must not exist")
    cases = cp.corpus()[: args.only] if args.only else cp.corpus()
    res = {"corpus_sha256": cp.digest(), "n": len(cases), "modes": {}}
    for mode, grammar in (("free", False), ("grammar", True)):
        rows = []
        for c in cases:
            out, secs, rss, st = infer(args, c, grammar)
            out2, _, _, st2 = infer(args, c, grammar)
            if st or st2:
                raise RuntimeError(f"inference failed for {c['case']}: wait statuses {st}, {st2}")
            s, l = strict(c, out), lenient(c, out)
            rows.append({"case": c["case"], "lang": c["lang"], "kind": c["kind"],
                         "out": out.strip()[:80], "secs": secs, "rss_kib": rss,
                         "exit": st, "deterministic": out == out2,
                         "strict_parse": s is not None, "strict_ok": validate(c, s),
                         "lenient_ok": validate(c, l),
                         "shown": select(c, out),
                         "wrong_if_unchecked": s is not None and not validate(c, s)})
        res["modes"][mode] = rows
    warm = [r["secs"] for r in res["modes"]["grammar"]]
    res["warm_grammar"] = {"p50": statistics.median(warm), "p95": pct(warm, .95),
                           "max": max(warm)}
    cold = []
    for _ in range(5):
        evict(args.model)
        _, secs, rss, status = infer(args, cases[0], True)
        if status:
            raise RuntimeError(f"cold inference failed: wait status {status}")
        cold.append((secs, rss))
    res["cold_grammar"] = [{"secs": s, "rss_kib": r} for s, r in cold]
    # cancellation: SIGTERM mid-generation, time to process exit
    c = dict(cases[0])
    cmd = [os.path.join(args.runtime, "llama-completion"), "-m", args.model, "-p",
           prompt(c, False), "-n", "400", "-no-cnv", "--temp", "0", "-t", "4",
           "-c", "512", "--ignore-eos"]
    lat = []
    for _ in range(5):
        p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                             env=dict(os.environ, LD_LIBRARY_PATH=args.runtime))
        time.sleep(1.0)
        t0 = time.monotonic()
        p.terminate()
        p.wait()
        lat.append(time.monotonic() - t0)
    res["cancel_secs"] = lat
    with open(args.out, "x", encoding="utf-8") as output:
        json.dump(res, output, indent=1, ensure_ascii=False)


if __name__ == "__main__":
    main()
