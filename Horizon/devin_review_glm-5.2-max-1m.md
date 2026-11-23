# Devin review — JARVIS RIP v1.1

**Reviewer:** Devin (GLM-5.2 Max 1M, 1M context)
**Date:** 2026-11-23
**Scope:** Review of `ROADMAP_v1.1.md`, `DEVIN_HANDOFF.md`,
`preregistration.v1.1.yaml`, `RecursiveIntelRoadmap.txt`, and
`engineer_eval_no_jarvis_context.txt`, with **independent verification of the
three cited arXiv papers** (the prior engineer review explicitly did not
verify them). Cross-referenced against `docs/decisions/ADR-0007` and
`docs/folding-validation-plan.md` in the Jarvis repo. No code or roadmap files
were modified; this is a review and a set of proposed edits.

**Stance:** This is not a rewrite of v1.1. v1.1 already incorporated the prior
engineer review's six critical gaps. My contribution is narrower and sharper:
one verified finding that reframes the W branch, plus the edits that follow
from it. Where v1.1 stands, I say so.

---

## 0. Verification of the three cited papers

The prior engineer review flagged "I haven't independently verified the cited
papers or the Qwen3.5 architecture claims." I did. All three are real and say
what v1.1 attributes to them:

| Paper | Verified | v1.1's claim | Matches source? |
|---|---|---|---|
| Self-rewriting (`arXiv:2611.21885`, v2, 18 Nov 2026, Fan et al.) | Yes | "open 27B model rewrites its own weights, +6.6 points, ability emerges with scale" | Yes |
| Model folding (`arXiv:2502.10216`, ICLR 2025, Wang et al.) | Yes | "data-free, fine-tuning-free compression via k-means neuron merging, MIT code at `nanguoyu/model-folding-universal`" | Yes |
| Byte distillation (`arXiv:2609.12303`) | Yes | "byte students start worse, surpass tokens with compute; strongest gains are extrapolated; no inference-cost win established" | Yes, including the caveats |

One bibliographic note for v1.1: the duplicate-citation bug the prior review
flagged (same arXiv ID for two entries) appears fixed in v1.1's bibliography.
The orphaned "Neuron-Level Sequential Editing" entry remains and should be
either linked to a real paper or removed.

---

## 1. Headline finding — the W subject is documented to fail in the source paper

This is the one thing the verification changed, and it is load-bearing.

The self-rewriting paper used **the exact Qwen3.5 family** the roadmap tests,
at all four sizes (2B, 4B, 9B, 27B), under one protocol. The paper's own
results table (verified against the v2 abstract, the braindetox explainer,
and the jiajunfan project page):

| Size | Self-edit on own weights | Damaged-copy repair | Located damage? |
|---|---|---|---|
| 2B | Made no edit | No edit attempted | No (never reads a weight) |
| **4B** | **Broke itself (held-out accuracy collapsed after its edit)** | Failed | No |
| **9B** | **Broke itself (47.5% → 0.0% → 0.0%)** | Failed | No |
| 27B | **Improved (+6.6 points, second protocol)** | Restored to 33% | Yes (layer 30) |

The emergence threshold sits between 9B and 27B in this family. The roadmap's
W subject is **the exact model the paper documents as breaking itself**.

This is not the same as the prior review's "unstated assumption #2" ("Edits
in dt_bias and A_log can affect meta-capability, not just task accuracy").
The prior review *guessed* the surface might be too narrow. The verification
*confirms* the model itself is documented to fail at the unconstrained task —
which makes the constrained-surface question the actual variable, not a
secondary concern. v1.1 buries this in unstated assumptions; it should be the
headline.

### Hardware constraint that follows

On the A4500's 20 GB VRAM, with the edit performed at BF16 (the paper's
precision):

- **27B BF16 ≈ 56 GB — does not fit.** The model that works is out of reach.
- 27B FP8 ≈ 28 GB — does not fit.
- 27B Int4 ≈ 14 GB — fits, but editing quantized weights is a different
  experiment the paper did not run; silently substituting Int4 changes the
  one variable (precision) the threshold finding depends on, and would need
  its own preregistration.
- 9B BF16 ≈ 18–20 GB — fills VRAM; no room for KV cache, edit mechanism, or
  the resident chamber.
- **4B BF16 ≈ 8 GB — fits comfortably, leaves ~12 GB for everything else.**

So on this hardware, **the W research is a 4B experiment or it is nothing.**
The model that fits is the one documented to fail.

---

## 2. What v1.1 gets right (preserved, not rewritten)

Briefly, because the prior review already covered the structural strengths
and v1.1 incorporated them:

- Bounded claim ladder (C0–C4) with explicit "finite study cannot establish
  exponential growth."
- Random + guided-grid + autonomous + frozen-editor arms with matched budgets.
- The 2×2 editor/target crossover with a frozen-G0-editor control running the
  same number of generations — the single design element that makes the
  recursion claim falsifiable.
- Four-way data split with sealed final, one opening per confirmatory study.
- Lineage as the statistical unit, treated as a random effect.
- The v1.0 → v1.1 revision is substantive: the 14-day plan is gone, the
  edit-surface probe (Gate 2) precedes lineage machinery (Gate 3), the
  three-way claim is walled off, the duplicate-citation bug is fixed.

These stand. My proposed edits below do not touch them.

---

## 3. Proposed edits to v1.1 (numbered, citing sections)

### Edit 1 — Add a "Scale-threshold finding" subsection at the top of §0 (Mandate)

Insert before the existing claim ladder. Proposed text:

> **Scale-threshold finding (verified, source paper).** The self-rewriting
> paper (`arXiv:2611.21885`, v2) tested the exact Qwen3.5 family at 2B, 4B,
> 9B, and 27B under one protocol. The 2B model made no edit; the 4B and 9B
> models broke themselves (held-out accuracy collapsed to 0% after their
> own submitted edits); only the 27B model improved (+6.6 points) and
> repaired a damaged copy. The emergence threshold in this family sits
> between 9B and 27B. The W subject (Qwen3.5-4B) is the exact model the
> paper documents as failing. This is not a caveat; it is the dominant prior
> on the central W hypothesis.

### Edit 2 — Reframe Gate 1's question (§4, Gate 1)

The current Gate 1 asks "Can model proposals beat random and fixed search?"
For 4B, the source paper says the answer is "no, they destroy capability."
The genuine question the roadmap's constrained surface introduces is
different. Proposed reframing:

> **Gate 1 (revised question): Does a constrained edit surface change the
> documented 4B outcome from "breaks itself" to either "safe no-op" or
> "small useful change"?** The paper's 4B result used an unconstrained
> edit surface — any tensor, any magnitude, any operation. This protocol's
> surface is bounded (`dt_bias`/`A_log` only, deltas 0.005–0.5, ≤4096
> scalars, ≤24 layers, whitelist-enforced, `torch.no_grad()`). The
> constraint is the genuine variable. Three outcomes, all informative:
> (a) catastrophic failure replicates the paper → the threshold is real and
> above 4B regardless of surface; (b) safe null → the surface is too narrow,
> triggering the preregistered fallback to attention/MLP/adapters; (c) small
> useful change → the constraint is the missing ingredient and the paper's
> unconstrained protocol, not the model size, was the cause. Outcome (c) is
> unlikely given the paper but not ruled out; it would be a novel finding.

### Edit 3 — Add a hardware-stop condition to the W branch (§4, after Gate 2's no-go path)

The current no-go path says "expand in preregistered new branch to bounded
attention/MLP scalar gates or gradient-free low-rank adapters; retain
initial negative finding." This loop has no named exit. Proposed addition:

> **Hardware-stop condition for W (independent of edit surface):** If, after
> the constrained `dt_bias`/`A_log` surface AND the preregistered fallback
> surfaces (attention/MLP scalar gates, low-rank adapters), no meta-effect
> is observed at 4B, AND the source paper's emergence threshold (≥27B) is
> confirmed as above this hardware's BF16 capacity, the W hypothesis is
> retired for this hardware configuration — not extended indefinitely to
> new surfaces. Retirement is a finding, not a failure; it bounds the
> claim's scope honestly. Reopening requires hardware that can hold a
> ≥27B model at BF16, or a separately preregistered quantized-edit study
> that does not silently substitute precision.

### Edit 4 — State the 27B/Int4 substitution risk explicitly (§2, Conditional after preflight)

The current text mentions Int4 only as a generic GPU option. Proposed
addition:

> **Quantized-edit substitution risk:** If W research is later moved to a
> 27B model at Int4 to fit VRAM, that is not "the same experiment at a
> smaller precision" — it is a different experiment, because the source
> paper's threshold finding was measured at BF16. Editing quantized
> weights may or may not preserve the emergence property; it is
> unmeasured. Any quantized-edit W run requires its own preregistration,
> its own pilot, and explicit acknowledgment that it does not replicate
> the paper's threshold test. Do not present a quantized-edit result as
> confirming or denying the paper's threshold finding.

### Edit 5 — Disambiguate the F branch from ADR-0007's LittleBit (§5, F branch intro)

The F branch cites `arXiv:2502.10216` (Wang et al., ICLR 2025, k-means
neuron merging — data-free, fine-tuning-free). This is a **different
mechanism** from the LittleBit/LittleBit-2 weight factorization
(Samsung, NeurIPS'25/ICML'26) referenced in the Jarvis repo's
ADR-0007 and `docs/folding-validation-plan.md`. They share the word
"folding" but not the math, the goal, or the testing path. Proposed
footnote to the F branch:

> **Notation:** The F branch's "folding" (Wang et al., ICLR 2025) is
> k-means neuron merging for data-free structural compression. It is
> distinct from LittleBit/LittleBit-2 (Samsung, NeurIPS'25/ICML'26),
> which factorizes weights into binarized low-rank ±1 factors for
> sub-1-bit portability. The two compose (merge neurons for structure,
> then sub-1-bit the result) but are separate research efforts. The
> Jarvis repo's `docs/folding-validation-plan.md` tests LittleBit
> specifically; it does not test this roadmap's F branch.

### Edit 6 — Move the 4B-vs-27B choice from "compute budget" to "open design decision" (§0 or a new §0.5)

v1.1 frames Qwen3.5-4B as a compute-budget choice. Given the verified
finding, it should be framed as a bet against the source paper's scaling
result. Proposed:

> **The 4B decision, stated honestly:** Qwen3.5-4B is chosen because it
> is the largest Qwen3.5 that fits comfortably at BF16 on the A4500
> with room for the edit mechanism. The source paper documents that 4B
> breaks itself under an unconstrained protocol. Proceeding at 4B is a
> bet that the constrained edit surface changes that outcome — not a
> neutral compute-budget default. The bet is registered, the
> counter-evidence is on file, and Gate 1 is where the bet is tested.

---

## 4. The "both" / multi-model question

The user asked whether to run "both" — a small always-on director plus a
larger self-improving specialist, eventually a library of specialists by
task. The vision is architecturally right for the Jarvis runtime: it is
ADR-0007's bicameral split (System 1 resident + System 2 on-demand)
extended to a specialist library. Horizon's own Level D ("branching
rather than destructive replacement... a fast folded model makes a better
diagnostic specialist") describes the same thing.

But applying "both" to the W *research* on this hardware does not give what
the vision wants:

- The model that works in the paper is 27B. It does not fit at BF16.
- 9B also breaks itself in the paper and fills the VRAM at BF16 — no room
  for the edit mechanism, KV cache, or the resident chamber. Running it at
  FP8 is a different experiment.
- 4B + 9B "both" doubles the compute for two models both documented to
  break themselves. That is two nulls at twice the cost, not a productive
  pair.

**The library vision is a Stage II runtime architecture, contingent on the
W research producing something worth housing.** Sequence:

1. W research determines whether self-improvement works at all on hardware
   available.
2. If yes, the improved models become the library's specialists.
3. The small resident chamber (ADR-0007 System 1) routes to them by task.
4. If no, the library stays empty and Jarvis runs on conventional models,
   which ADR-0007 Stage 1 already supports.

The vision is sound. The library housing should not be built before there
is a model worth housing.

---

## 5. What stands unchanged in v1.1

- The claim ladder (C0–C4) and the "finite study cannot establish
  exponential growth" framing.
- The four arms (random, guided-grid, autonomous, frozen-editor) and the
  2×2 crossover.
- The four-way data split and the sealed-final, one-opening rule.
- Gate 0 (hardware preflight) as the gating step for everything — including
  the Windows/Triton kernel concern the prior review raised, which v1.1
  correctly defers to Gate 0.
- The F branch's folding targets (5/10/20% actual parameter reduction,
  measured latency) and the comparison against quantization/pruning/
  distillation baselines.
- The B branch's three-way comparison (equal data, equal train FLOPs, equal
  inference cost) and the EOT probability-conservation unit tests.
- The "no premature three-way claim" rule (§5 combination logic) and the
  requirement for eight factorial arms before any F+W+B synergy claim.
- The Stage II memory separation (episodic immutable / semantic versioned /
  procedural promoted-after-testing) and the human-review gate on
  promotions.
- The governance rules (pinned-commit audit, no sealed data in public git,
  separate evaluator, hash-verified patch manifests).

These are the bones of the protocol and they are correct. My proposed
edits sharpen the W framing without disturbing them.

---

## 6. Open questions for the author

1. **Adopt the reframed Gate 1?** It changes the question from "can 4B
   self-edit" (the paper answered: no) to "does the constrained surface
   change the documented failure" (unanswered, genuinely novel). This is
   the single most consequential edit.
2. **Add the hardware-stop condition?** It names an exit for the W branch
   independent of edit surface. Without it, the fallback loop can extend
   indefinitely on hardware that may be below the threshold by construction.
3. **Drop or keep "both" for the W research?** My recommendation is one
   model run well (4B BF16). Adding 9B doubles cost for a model also
   documented to fail and that barely fits. The library vision is a
   Stage II runtime decision, not a W-research design.
4. **Preregister a quantized-edit W study as a separate protocol?** If
   you eventually want to test 27B at Int4, that needs its own v1.x
   preregistration — it is not a continuation of the BF16 W study. I'd
   hold this until the 4B BF16 result is in.
5. **Roll these edits into a v1.2, or keep this review as a standalone
   addendum?** The prior pattern (engineer review → v1.1) suggests the
   author rolls approved edits into the next version. I have not edited
   `ROADMAP_v1.1.md`; that is the author's call.

---

## 7. Summary judgment

v1.1 is a well-designed protocol to test a hypothesis that is probably
false on this hardware, with the right structure to produce a useful null
if it is. The verified scale-threshold finding raises the prior on "false"
substantially — but does not change the protocol's value, because the
constrained-surface question is genuinely novel and three informative
outcomes remain possible. The protocol's strength is that it can absorb
this finding without breaking: Gate 1 already exists to screen for
exactly this. The only structural addition I consider necessary is the
hardware-stop condition (Edit 3), without which the preregistered
fallback loop can extend indefinitely past the point of honest retirement.

The "Marvel Jarvis" vision the user holds — a bicameral, always-on,
specialist-library assistant — is the right long-term shape for the
runtime. It is contingent on W producing something, and on this
hardware W is a 4B experiment with a high prior on null. The vision does
not require W to succeed to be worth pursuing; ADR-0007's Stage 1
(conventional-quant bicameral) delivers always-on + bicameral today, and
the specialist library defers until there is a specialist worth housing.
