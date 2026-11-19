# Offline coach language alternatives (#79): no-go for this model and scope

Status: **no-go for production integration** of the tested Qwen3-0.6B Q8_0
arm; **no-need** for a learned phrase selector. Fluent templates stay the
default. The verdict covers exactly what was measured below (one model, one
quantization, one task shape, one named CPU). Learner readability was **not**
measured and no learner observation exists or was simulated. Issue #79's
learner item stays open; nothing here is a claim about clarity or learning.

## Comparison run (2026-11-19)

Inputs: the retained Q8_0 file and llama.cpp `ffc87c3` build described below
(hashes rechecked), CPU only, 4 threads, 512-token context, greedy decoding,
Intel Core i9-10900K. Harness and corpus: `scripts/research79/` (disposable,
not in the gate; raw results stay in `grand-coach-data/studies/`). The corpus
was frozen and committed before any model output: 43 evidence records (4
boundary, 19 supported, 5 uncertain, 5 conflicting, 4 incomplete, 6
adversarial) rendered into German and English, 86 cases, SHA-256
`9aeb384857022c7311558312fe781a7c6924f4310fea44c8837a42f81afcc7ab`.
Concepts: remaining trumps (exact/estimated count) and proven void (suit).

**Task.** From the evidence, pick one phrase ID and its parameter. Both arms
render the same Fluent-style sentence from a validated `(ID, parameter)`, so
wording is identical by construction; only the *selection* differs.
**Templates arm**: the deterministic rule on the structured fields. It is the
reference, so it is correct on all 86 cases by definition; that shows
determinism, not an independent accuracy result. **Model arm**: Qwen3-0.6B
chooses the pair; a post-check accepts it only if it equals the
evidence-supported pair, otherwise the template result is shown (fallback).

| Model mode | Output parses as `ID|PARAM` | Matches evidence | Wrong output if shown unchecked | Deterministic (2 runs) |
|---|---|---|---|---|
| Free text, `/no_think` | 0/86 (every output starts with an empty `<think>` block and a malformed answer) | 0/86 | n/a | 86/86 |
| GBNF-grammar constrained | 86/86 | 19/86 (DE 8/43, EN 11/43) | 67/86 | 86/86 |

By kind with grammar (matches/cases): boundary 3/8, supported 12/38,
uncertain 0/10, conflicting 4/10, incomplete 0/8, adversarial 0/12. Typical
failures were wrong digits (e.g. always `10`/`12` for other counts), picking
`count_exact` for unknown or missing evidence, and following injected notes.

**Enforcement and fallback.** The grammar guarantees syntax only; it did not
make the model preserve facts. The post-check (equality with the evidence
pair) rejected all 67 wrong grammar outputs and all 86 free outputs, so a
shown sentence was never wrong in this corpus, and the system fell back to
templates in 78% of cases (grammar) or 100% (free). That is corpus evidence
only, not a proof. The check lives in the harness; no product validator,
in-process per-token cancellation, or seeded fallback path was implemented.
Process-level cancellation (SIGTERM during a 400-token generation) ended the
process in about 11-13 ms. In-process token cancellation is untested.

**Latency and memory** (single short request per process, `wait4` max RSS,
page cache warm unless stated): grammar mode wall time p50 0.72 s, p95 0.85 s,
max 1.02 s over 86 requests; free mode p50 0.84 s, p95 1.74 s, max 1.99 s
(16-24 token cap). Cold (file pages evicted with `POSIX_FADV_DONTNEED`, 5
runs): 0.94-1.31 s. Peak RSS about 0.75 GB (750,000-773,000
KiB) per process, dominated by the mapped 639 MB weights. Incremental RSS of
the application was not isolated; the whole process figure is an upper bound.
Thread scaling, Windows and slower CPUs were not measured, so the targets
still hold only for this machine.

**Offline packaging and licences.** Weights 639,446,688 bytes plus about
15 MB of shared libraries; this is above the D-63 10 MB model cap by a factor
of ~64 and needs a new decision, not a quiet exception. The retained Linux
build links `libssl.so.3` via `libllama-common` and ships RUNPATH-relative
shared objects; no Windows build, static link or isolated offline package was
produced. Licences recorded: llama.cpp MIT; Qwen publisher declares
Apache-2.0 for GGUF repository and base card. Tokenizer source revision and
conversion procedure remain unaudited (file metadata only, below).

**Decision.** No-go. The model cannot improve on the template arm for this
task: the ID and parameter are already determined by structured evidence, the
model reproduced them correctly in 22% of cases, and every correct answer is
identical to the template output. It adds about 0.75 GB RSS, a 639 MB
package, a native runtime and unvalidated provenance, and wording (the only
thing learners could perceive) does not change. A larger or fine-tuned model,
free paraphrase with a richer enforcement design, or a different task would
need a new issue with evidence; none is justified now. Reproduce with
`python3 -B scripts/research79/run.py --runtime DIR --model FILE --out OUT`
(about 10 minutes).

## Bounded development attempt

The prohibition on runtime downloads applies to the delivered program, not to
pinned development inputs. The previous assertion of a blanket download ban
was incorrect. On an Intel Core i9-10900K, the official
[Qwen3-0.6B-GGUF](https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/tree/23749fefcc72300e3a2ad315e1317431b06b590a)
Q8_0 file was downloaded at revision `23749fefcc72300e3a2ad315e1317431b06b590a`:
639,446,688 bytes, SHA-256
`9465e63a22add5354d9bb4b99e90117043c7124007664907259bd16d043bb031`.
This meets the weight-size target alone, not the package or memory targets.
The official model repository states Apache-2.0; tokenizer provenance and
licence still need verification separately.

The retained Q8_0 file was rehashed on 2026-11-19 and still matches that
SHA-256, including the hash published on the pinned [weight file page](https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/23749fefcc72300e3a2ad315e1317431b06b590a/Qwen3-0.6B-Q8_0.gguf).
The retained llama.cpp checkout reports the revision above and its
`LICENSE` is MIT. The pinned [GGUF repository](https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/tree/23749fefcc72300e3a2ad315e1317431b06b590a)
identifies Qwen as publisher and labels the model Apache-2.0; the
[base model card](https://huggingface.co/Qwen/Qwen3-0.6B) also declares
Apache-2.0. These declarations do not independently establish the embedded
tokenizer's exact source revision or its licence, nor the conversion and
quantization procedure for these bytes. The retained Linux executable uses
shared libraries: its presence does not prove a linked, standalone package
or Windows support. No model was downloaded for this recheck.

Inspection with that checkout's `gguf-py` reader found 197 Q8_0 and 113 F32
tensors, quantization format version 2, and embedded tokenizer tokens,
merges and chat template. The tokenizer metadata says `gpt2` with `qwen2`
pre-tokenization, 151,936 tokens and 151,387 merges. These are file contents,
not proof of the tokenizer source revision, conversion procedure or separate
licence. The pinned GGUF repository includes an [Apache-2.0 licence file](https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/23749fefcc72300e3a2ad315e1317431b06b590a/LICENSE);
the runtime licence and the embedded tokenizer remain separate audit items.

A disposable [llama.cpp](https://github.com/ggml-org/llama.cpp/tree/ffc87c3a7989cb657641a6459a186f303e176af8)
checkout at `ffc87c3a7989cb657641a6459a186f303e176af8` has an MIT licence.
The toolbox build command failed with `cmake: command not found` (exit 127).
A temporary CMake and host compiler were then used; a first configuration
failed because ccache could not write its cache, and disabling ccache allowed
configuration. The attempted `llama-cli` build target failed because this
revision has no such target. The `llama-completion` target built successfully as a replacement. No product dependency or model was added to the repository.
These setup attempts do not establish a fully offline, linked standalone
package on Linux or Windows.

One unconstrained English smoke prompt supplied only `remaining_trumps=3`,
`certainty=exact` and the allowed IDs `count_exact` and
`insufficient_evidence`. With `/no_think`, greedy decoding, four CPU threads,
a 512-token context and 16 output tokens, the model returned `count_exact`
but also emitted `<think>` tags. A strict ID-only validator must reject that
output; no validator was implemented or tested here. One process took 0.80 s
wall time and peaked at 750,096 KiB RSS (`/usr/bin/time -v`); this single
uncontrolled smoke run is neither cold/warm p95 nor incremental application
RSS. It does not establish factual preservation, bilingual quality or a
performance target.

## Approach 1: deterministic Fluent templates (current)

`skat-coach` already emits only structured data: text keys, parameters,
`Concept`, `Amount`, `Judgement`, `Reason` and `Basis`
(`crates/skat-coach/src/lib.rs`). `assets/i18n/{de,en}.ftl` render them, with
identical key sets in both languages. This arm is deterministic and offline;
the existing resources add no model package. Templates still require evidence
and translation validation and do not guarantee factual correctness. Its
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

## Approach 3: one small pretrained multilingual model (not compared)

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

The pinned runtime and model-repository licences and the weight hash are
recorded above. The tokenizer and quantisation provenance and licences still
need a separate audit before inclusion. Feasibility of fully offline bundling is plausible but unproven:
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
| Model arm vs templates (86 bilingual cases) | 19/86 correct with grammar, 0/86 free; every error caught and replaced by fallback |
| Warm p95 / RSS | 0.85 s (grammar) / about 0.75 GB process peak, CPU named above |
| Learner readability | Not measured; no learner data exists |
| Go for production integration | No. No follow-up implementation issue |

Fluent templates remain the only coach language path. Actual learner
readability of the template wording stays an open, human-only question.
