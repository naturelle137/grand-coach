# Offline coach language alternatives (#79): inconclusive

Status: **inconclusive** for the learned-model arm; **no-need** for a
learned phrase selector. Fluent templates stay the default. No runtime,
weights or tokenizer were downloaded, added or run. No benchmark, generation,
fine-tuning or learner session took place, so no timing, RSS or readability
figure exists. Issue #79 stays open with its checklist unticked.

## Why nothing was measured

The issue forbids Ollama, external services and runtime downloads, and gives
no authority to fetch weights. This worktree has neither a llama.cpp build
nor a Qwen3 GGUF. Adding a linked llama.cpp crate also needs a licence and
lockfile audit (`docs/third-party.md`, D-42). Measuring without these would
mean inventing numbers, which the issue prohibits. Parameter count says
nothing about speed on a given CPU, so the 2 s, 1 GiB and 2 GiB targets stay
**unmeasured** and unreported as met or missed.

## Approach 1: deterministic Fluent templates (current)

`skat-coach` already emits only structured data: text keys, parameters,
`Concept`, `Amount`, `Judgement`, `Reason` and `Basis`
(`crates/skat-coach/src/lib.rs`). `assets/i18n/{de,en}.ftl` render them, with
identical key sets in both languages. This arm is deterministic, offline and
free of invented facts by construction, and it adds no package size. Its only
weakness is wording monotony, which is a readability question that only
learners can answer. Improving it means editing `.ftl` text, with no
runtime risk.

## Approach 2: selection among vetted phrase/lesson IDs

The coach already selects the lesson (`pick`, D-58) and the stage
(`Stage::next`, D-49) from evidence by deterministic rules. A learned
selector would only choose between pre-written variants of the same fact.
That could be a seeded choice among Fluent variants, with no model. The
expected gain is wording variety, and the cost is a larger translation
surface to keep consistent. **Result: a learned selector is not needed.**
If variety is wanted later, a seeded variant choice reproduces under the
existing determinism rule.

## Approach 3: one small pretrained multilingual model (unmeasured)

Candidate per the issue: Qwen3-0.6B, non-thinking mode, in a linked local
runtime (llama.cpp). Design constraints a future proof must meet:

- **Output**: a grammar-constrained list of evidence or phrase IDs and
  validated parameters, rendered by Fluent. Grammar constraints guarantee
  syntax, not truth, so a post-check must verify every ID and parameter
  against the supplied evidence and fall back to Approach 1 on any mismatch.
  Free factual paraphrase needs an enforcement design that does not exist
  and is out of scope.
- **Input**: only the permitted view and structured supported evidence
  (D-49, D-60). Never hidden state. The model cannot choose or invent the
  human line.
- **Layering**: inference needs file IO and a native runtime, so it belongs
  in a `gui-*`/`skat-app`-adjacent crate, never below `skat-coach`.
- **Budget**: fixed prompt and token caps, a cancellation point per token,
  and a seeded sampler, or greedy decoding for determinism.

Facts recorded here are **not re-verified in this job**: llama.cpp is MIT
licensed, and the Qwen3 model card lists Apache-2.0. Both are allowed
licences (D-42), but exact weights, tokenizer and quantisation-source
licences, hashes and sizes must be audited from the actual files before any
inclusion. Feasibility of fully offline bundling is plausible but unproven:
it needs a C/C++ toolchain on Linux and Windows 11 CI, a new notice entry,
and weights outside git or under the D-63 rule (D-63 caps model files at
10 MB, so a 0.6B model needs a new decision).

## Frozen evaluation protocol for a later attempt

1. Corpus: de/en cases of supported, uncertain, boundary, conflicting,
   incomplete and adversarial evidence, covering all coach stages. It is
   frozen before any model output is seen.
2. Same evidence into all arms. Measure factual and parameter preservation,
   unsupported claims, visibility leakage, exact versus estimated wording,
   cancellation and determinism.
3. Report zero violations as corpus evidence only.
4. Timing: cold and warm, p95 for short output under a fixed cap, plus
   package size and incremental RSS, on a *named* CPU, no more than two hours
   in total.
5. Learner clarity from real learners only, never agent-filled, and separate
   from strategic correctness.

## Conclusion and next step

| Question | Result |
|---|---|
| Learned phrase selector | Not needed; seeded Fluent variants suffice if variety is wanted |
| Model arm quality, latency, size, RSS, readability | Not measured: inconclusive |
| Go for production integration | No. No follow-up issue is justified yet |

Unblocking requires a maintainer decision to allow a pinned weights download
and a linked runtime in a throw-away prototype crate. Until then, Fluent
templates remain the only coach language path.
