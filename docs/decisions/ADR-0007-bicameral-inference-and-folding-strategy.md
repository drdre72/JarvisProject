# ADR-0007: Bicameral inference and model-folding strategy

**Status:** proposed · **Date:** 2026-11-21

## Decision

Adopt a **bicameral** inference architecture and a **role-gated folding
policy**, rather than compressing one model to fit everything.

- **System 1 — always-resident chamber:** a small model kept in memory for
  wake confirmation, intent routing, and dispatch. It must stay loaded between
  turns so the assistant is always-on. Quality bar is *reliable routing*, not
  deep reasoning. The Rust supervisor (ADR-0006) owns the two-chamber routing
  and the resident-chamber lifecycle, since it already owns the inference
  lifecycle under `[rust_core] inference_relay`.
- **System 2 — on-demand reasoning chamber:** the current
  `models/qwen2.5-7b-instruct-q4_k_m.gguf` (ADR-0002), loaded only for
  substantive generation. Stays at a near-lossless conventional quant;
  sub-1-bit folding is **not** applied here, because the intent gate
  (`scripts/decision_controller.py`, `scripts/semantic_intent.py`,
  `reasoning.generate_structured_response` at `--temp 0`) depends on a
  trustworthy classifier and sub-1-bit degrades knowledge faster than
  fluency (see Context).

Folding policy, by the three distinct meanings of "folding":

1. **Deterministic context-fold pattern** (à la `Middlewatch/context-fold`):
   adopt the *pattern* — mask stale tool observations to verbatim digests with
   sha256-verified recall, never a paraphrase, no model call — into the Python
   layer when multi-step tool use arrives. Adoptable now with no dependencies
   and no training; fits ADR-0001 (offline) and the memory/imagination/facts
   separation rule exactly.
2. **LittleBit / LittleBit-2 weight folding** (Samsung, NeurIPS'25 / ICML'26):
   permitted **only for the System 1 resident chamber**, and only when three
   gates clear: (a) a production CPU inference path on Windows/MSVC or a
   `littlebit-gguf` path inside llama.cpp's 1-bit tensor type; (b) QAT of a
   small model (0.6B–1.7B) on the local GPU; (c) license clearance against
   Samsung's CC BY-NC 4.0 (or use an MIT reimplementation). It is **never** the
   reasoning brain.
3. **Context-Folding / FoldGRPO** (ByteDance, `arXiv:2510.11967`): rejected
   as-shipped — it requires an OpenAI grading key, a networked search server,
   and an RL-trained model, all violating ADR-0001. The *branch/return* agent
   pattern is borrowable later as orchestration, without the RL training.

Adoption is staged and reversible, mirroring ADR-0006's shadow-first shape:

- **Stage 0 — pattern only:** deterministic context-fold in Python, no new
  model, no new binary. Builds the multi-step substrate.
- **Stage 1 — bicameral on the existing stack:** introduce a small
  conventional-quant System 1 (e.g. Qwen3-0.6B/1.7B at Q4_K_M or Q2_K, <1 GB)
  alongside the current Qwen2.5-7B. Realizes always-on + bicameral today with
  no quality gamble on the reasoning path.
- **Stage 2 — fold the resident chamber:** replace the System 1 conventional
  model with a locally QAT-trained LittleBit model only after the three gates
  clear and the intent gate is re-verified on the folded model.

## Context

- **Three "foldings" are routinely conflated** in the source material and must
  be separated: Context-Folding folds *agent history* (needs RL training,
  offline-hostile); LittleBit folds *weights* (needs QAT, costs capability);
  context-fold (Middlewatch) folds *stale tool output* (deterministic, no model
  call). Only LittleBit makes anything portable; only the agent foldings make
  anything more capable.
- **The quality cost is real and is documented by an independent benchmark**
  (`christopherrobertbrooks-tech/littlebit-bench`, same evaluator as llama.cpp
  quants). Qwen3-0.6B: fp16 48.4% zero-shot; **llama.cpp Q4_K_M 45.6%**
  (near-lossless — Jarvis's current class); Q2_K 41.9%; **LittleBit-2 @ 1.0
  bpw 38.6%**, barely above 35.7% chance and *below* chance on `arc_challenge`
  and `openbookqa`. Headline "31×" is the transformer body only; embeddings +
  lm_head stay FP16 and dominate at small scale (saved 1.0-bit 0.6B is 647 MiB
  vs 1.5 GB fp16 — ~2.3× total, not 16×). The 0.6B model is the *hard* case;
  7B+ tolerate sub-1-bit better, but multi-GPU QAT is required for the open
  recipes. Fluency recovers faster than knowledge — the opposite of what a
  reliable classifier wants.
- **Jarvis constraints filter every option:** offline at runtime (ADR-0001)
  rules out FoldAgent as-shipped; Windows / CPU llama.cpp today means
  `GeniusDevam/littlebit.cpp` (MSVC *planned*, not done) is not yet usable,
  leaving `littlebit-gguf` as the most promising native path; the intent gate
  cannot tolerate below-chance reasoning; Samsung's CC BY-NC 4.0 must be
  checked against Jarvis's own license before any adoption.
- **Local GPU changes the small-model story.** The development machine has an
  NVIDIA RTX A4500 (~20 GB). `littlebit-bench` trained Qwen3-0.6B @ 1 bit on a
  single 32 GB V100 in ~5 h 40 m; `mgiachetti/little-bit` trained on an 8 GB
  GTX 1080 with gradient checkpointing. So **a folded small resident model is
  locally trainable**; a folded *massive* (7B+) model is not, on one 20 GB card,
  with current open recipes.
- **Wearable target:** a custom 3D-printed wearable chosen for thermal and
  form-factor constraints ("keep things cool," "sleek not to hinder
  movement"). That intent implies an embedded-class compute budget, not a
  pocket-GPU. Two architectures follow: (a) on-device brain (LittleBit's
  footprint/add-sub matmuls genuinely matter) or (b) **tethered** — thin
  wearable as capture/display, brain on a phone/PC the user already carries.
  (b) is the lower-risk path to "always-on without melting," and is the
  default until the wearable's actual compute budget is fixed. This ADR does
  not pick between them; it keeps the inference brain portable enough that
  either binding is still possible.
- **ADR-0002** set Q4_K_M as near-lossless and made the model swappable by a
  filename constant; this ADR keeps that for System 2. **ADR-0006**'s
  supervisor already owns `llama-completion.exe` lifecycle under
  `inference_relay` and is the natural owner of chamber routing and the
  resident model's residency policy.

## Consequences

- **Capability is an architecture/training problem, not a compression
  problem.** Folding makes Jarvis *portable*; it does not make Jarvis smarter.
  Marvel-grade ability needs tool use, long-horizon reasoning, and memory —
  addressed by Stage 0's pattern and future tool work, not by any quant scheme.
  This ADR is explicit that compression is not the capability lever.
- **Stage 1 realizes bicameral + always-on with zero new dependencies and zero
  quality gamble** on the reasoning path — the current Qwen2.5-7B stays
  exactly as ADR-0002 fixed it. The only new asset is a small conventional GGUF.
- **Stage 2 is gated, not scheduled.** No LittleBit binary, no QAT run, and no
  CC BY-NC code enters the repo until the three gates clear and the intent
  gate is re-verified on the folded model. A failed Stage 2 falls back to the
  Stage 1 conventional-quant resident model with no regression.
- **No runtime network surface is added** (ADR-0001 preserved): FoldAgent's
  search server and grading API are out; the context-fold pattern and any QAT
  are offline/local. Build-time GPU staging for QAT is an approved step like
  the wheelhouse, not a runtime dependency.
- **The tethered-wearable option stays open.** Keeping System 1 small and
  System 2 a single swappable GGUF means the brain can bind to a phone/PC
  (tethered) or to the wearable itself (on-device) without an architecture
  change — the decision defers to the wearable's measured compute budget.

## Alternatives rejected

- **Fold one massive model to sub-1-bit for everything:** rejected on three
  grounds — knowledge degrades below chance on reasoning tasks at 1 bpw, 7B+
  QAT needs multi-GPU the local card cannot provide, and CC BY-NC blocks
  commercial-style distribution. Trades away the reasoning chamber for a
  footprint win that a small conventional System 1 already delivers.
- **FoldGRPO / Context-Folding as-shipped:** violates ADR-0001 (OpenAI grading
  key + networked search server) and needs an RL-trained model unavailable
  offline. Only the branch/return *pattern* may be borrowed, later.
- **LLM-summarization context compaction:** loses exactly what matters
  (identifier trails, error lines), fabricates "ground truth," and violates
  the memory/imagination/facts separation rule. The deterministic,
  no-model-call context-fold pattern is chosen instead specifically because
  it cannot hallucinate.
- **Stay single-chamber:** leaves no path to an always-on resident brain and
  no answer to the wearable thermal constraint. Bicameral is the smallest
  change that opens both.
- **Compress System 2 to Q2_K / IQ-quants to "save room":** the benchmark
  shows Q2_K already costs ~6.5 zero-shot points and IQ2_XXS/IQ1 are broken
  (PPL 600–4000). Not worth it for the chamber that carries reasoning.
