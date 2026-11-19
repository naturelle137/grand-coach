# Offline coach language alternatives (#79): inconclusive

Status: **inconclusive** for the learned-model arm; **no-need** for a
learned phrase selector. Fluent templates stay the default. A pinned model and runtime were obtained for a disposable development attempt.
One CPU smoke test was run; no comparative benchmark, fine-tuning or learner
session took place, so p95, incremental RSS and readability remain unknown. Issue #79 stays open with its checklist unticked.

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

## Availability recheck (2026-11-19)

The bounded comparison was retried with already-available inputs only. The
retained Qwen3-0.6B Q8_0 file and the disposable llama.cpp checkout and
`llama-completion` build named above no longer exist on this machine (file
search for `*.gguf`, `Qwen3-0.6B*`, `llama-*` executables and `libllama*`
found no candidate). The only other GGUF files belong to an unrelated project,
are different and much larger models, and have no recorded hash or provenance
here. No model was downloaded, so the model arm cannot be run. The result
stays **inconclusive** (not a measured no-go): production integration remains
**no**, and Fluent templates remain the only coach language path. A later
attempt needs an explicit decision to re-obtain the pinned inputs listed above.

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
| Model arm comparison, p95, incremental RSS, readability | Not measured: inconclusive |
| Go for production integration | No. No follow-up issue is justified yet |

Complete the bounded prototype, corpus and measurements, audit the exact
tokenizer and quantisation provenance, and obtain actual learner readability
observations. Fluent templates remain the only coach language path.
