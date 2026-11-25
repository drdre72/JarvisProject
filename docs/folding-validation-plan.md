# Folding-validation lab plan

Companion to ADR-0007 (bicameral inference and model-folding strategy).
This is a **plan for review**, not a decision and not built code. Nothing here
is committed until you approve it; the venv and code are not created yet.

## Purpose

Empirically validate ADR-0007's Stage 2 gate (c): *"intent gate re-verified
on the folded model."* The papers give us perplexity; what Jarvis actually
depends on is operational behavior. The lab measures the latter, not the
former. Perplexity is a proxy; this plan tests the things the proxy stands
for.

## What this does NOT test (stated up front, to scope honestly)

- **Packed-kernel inference speed.** `mgiachetti/little-bit` stores FP32
  latents and has no bit-packed kernel. Speed-of-light is the HDL lab's job
  (Sims2026), downstream. Here we measure quality and behavior, not packed
  throughput.
- **Capability gain.** Compression does not make Jarvis smarter
  (ADR-0007). This lab cannot and does not test capability — only whether
  folding preserves *enough* of what Jarvis already needs.
- **7B+ folding.** Open QAT recipes for 7B are multi-GPU DeepSpeed
  (`SamsungLabs/LittleBit` README). One 20 GB A4500 cannot do 7B QAT in
  reasonable time. The lab targets 0.6B–1.7B, which is the resident-chamber
  size class anyway (ADR-0007 System 1).
- **FoldGRPO / Context-Folding agent training.** Offline-hostile
  (ADR-0007). Out of scope.

## Environment

New, dedicated, sibling to Sims2026 and JarvisProject — does **not** pollute
the Sims2026 HDL venv or the JarvisProject app venv.

```
C:\Users\Shadow\Documents\folding-lab\
  .venv\                          # PyTorch 2.8 + cu124 (Python 3.11)
  littlebit-ref\                  # clone of mgiachetti/little-bit (MIT)
  Jarvis-checks\                  # the operational test harness (ours)
    test_intent_gate_on_folded.py
    test_capability_refusal.py
    test_comm_failure_contract.py
    bench_latency_rss.py
    eval_matrix.py                # runs the grid, writes results/<run>/*.json
  baselines\                      # llama.cpp Q4_K_M / Q2_K of the same models
  results\                        # JSON + matplotlib plots
  PLAN.md                         # symlink/copy of this doc
```

- Reference implementation: **`mgiachetti/little-bit`** (MIT, pure PyTorch,
  from-scratch, has the Qwen3 attention-sink fix, trains on 8 GB — A4500's
  20 GB is comfortable). Chosen over `SamsungLabs/LittleBit` (CC BY-NC,
  non-commercial — license risk) and `GeniusDevam/littlebit.cpp` (Linux/WSL,
  MSVC not done).
- Baseline methodology borrowed from `christopherrobertbrooks-tech/littlebit-bench`:
  same evaluator for every row, llama.cpp quants dequantized and scored
  identically to LittleBit. We adopt the *structure*; the CC BY-NC license
  on that repo means we reimplement the scoring, not copy it.
- Hardware: NVIDIA RTX A4500, 20 GB VRAM, driver 621.14 (CUDA 12.x ready).

## Test matrix

```
              | intent_gate | cap_refusal | comm_failure | latency | RSS | PPL
------------- | ----------- | ----------- | ----------- | ------- | --- | ---
Qwen3-0.6B Q4_K_M (base)    |     ✓    |     ✓    |     ✓    |    ✓   |  ✓  |  ✓
Qwen3-0.6B Q2_K             |     ✓    |     ✓    |     ✓    |    ✓   |  ✓  |  ✓
Qwen3-0.6B LittleBit 1.0bpw |     ✓    |     ✓    |     ✓    |    ✓   |  ✓  |  ✓
Qwen3-0.6B LittleBit 0.55  |     ✓    |     ✓    |     ✓    |    ✓   |  ✓  |  ✓
Qwen3-1.7B Q4_K_M (base)    |     ✓    |     ✓    |     ✓    |    ✓   |  ✓  |  ✓
Qwen3-1.7B LittleBit 1.0bpw|     ✓    |     ✓    |     ✓    |    ✓   |  ✓  |  ✓
```

Rows are **fixed before any training runs** (anti-p-hacking). All cells are
reported, not just the passing ones.

## The operational tests (the part the papers don't give)

These feed the folded model the *actual* prompts Jarvis uses and check
whether the responses still work. This is the gate ADR-0007 requires and
that nobody has published.

1. **`test_intent_gate_on_folded.py`** — replicates
   `reasoning.generate_structured_response`: runs the folded model at temp 0
   with the same JSON schema, on the same intent prompts the live
   `decision_controller` sees (capability questions, purpose questions,
   dialogue, unclear). Measures: JSON parse success rate, intent-label
   match rate vs the Q4_K_M baseline, off-schema drift. **A folded model
   that emits malformed JSON or mislabels intent breaks the intent gate
   silently — this is the highest-priority test.**

2. **`test_capability_refusal.py`** — probes with requests for unregistered
   capabilities (legacy macOS commands, network actions, things
   `capability_registry.py` does not declare). Measures: does the folded
   model falsely claim abilities (the "knowledge degrades faster than
   fluency" failure mode from `littlebit-bench`)? Refusal of unknown
   capabilities is a **hard gate** — any false capability claim fails the
   model for the resident chamber.

3. **`test_comm_failure_contract.py`** — feeds ambiguous/empty/noisy inputs
   and checks whether the folded model classifies the failure mode honestly
   per `AGENTS.md`'s typed-outcome categories, vs confabulating an acoustic
   cause or inventing a reason. The sub-1-bit "fluent but shallow" failure
   mode is the worst case for this contract.

4. **`bench_latency_rss.py`** — resident RSS (the whole point of folding),
   cold-start time, per-turn latency, tok/s. Compared to Q4_K_M on the
   same model. Folding should win on RSS and residency, not necessarily on
   tok/s (no packed kernel in the reference).

5. **PPL** — WikiText-2 + C4, same evaluator as `littlebit-bench`, as a
   sanity check against the published numbers (not the decision metric).

## Pass/fail criteria (pre-registered, before any run)

These are the conditions under which a folded model is *acceptable as the
System 1 resident chamber* per ADR-0007. Failing any hard gate disqualifies
that bpw/model cell for the resident role; soft gates are reported and
weighed.

| Gate | Type | Threshold |
|---|---|---|
| Intent-gate JSON parse success | hard | ≥ 95% |
| Intent-label match vs Q4_K_M baseline | hard | ≥ 90% |
| Capability refusal (no false claims) | hard | 100% |
| Comm-failure: no confabulated acoustic cause | hard | 100% |
| Resident RSS | soft | < 1 GB |
| Per-turn latency vs Q4_K_M | soft | ≤ 2× (folding should win, not lose) |
| WikiText-2 PPL vs Q4_K_M | report only | not a gate |

Pre-registration: this table is fixed before training. If a cell fails, it
fails — no iterating on the threshold after seeing results. A failed Stage
2 falls back to the Stage 1 conventional-quant resident model with no
regression (per ADR-0007).

## Workflow

1. Create `folding-lab\` + venv (PyTorch 2.8 + cu124).
2. Clone `mgiachetti/little-bit` into `littlebit-ref\`.
3. Pull Qwen3-0.6B and Qwen3-1.7B; create Q4_K_M and Q2_K GGUFs for
   `baselines\` (llama.cpp quantize).
4. Write `Jarvis-checks\` harness (tests 1–4) + `eval_matrix.py`.
5. Short LittleBit-2 training runs (~50M tokens each, a few hours per cell
   on the A4500 — matches `littlebit-bench` "night 1" budget, which is a
   *floor* not a ceiling per that repo's own note).
6. Run the full matrix; write `results\<run>\*.json`.
7. Generate the comparison plot (matplotlib) and the verdict table.
8. Decision: does any folded cell pass all hard gates AND beat Q4_K_M on
   RSS? If yes → candidate for ADR-0007 Stage 2. If no → stay Stage 1.

## Anti-p-hacking controls

- Matrix and thresholds fixed before training (this doc, approved by you).
- All cells reported, including failures.
- One Tree-like arrangement of the test prompts (no post-hoc prompt
  selection).
- Baseline (Q4_K_M) evaluated with the identical harness, identical
  prompts — no favorable treatment of the folded rows.
- Training budget capped and recorded per cell; no "train until it passes."

## Open questions for you before I build

1. **License:** `mgiachetti/little-bit` is MIT — fine for our use. Confirm
   you're OK with a PyTorch+CUDA venv on this machine (it's a ~3 GB install).
2. **Models:** Qwen3-0.6B and 1.7B are the right resident-chamber size class.
   OK to pull both, or start with just 0.6B?
3. **Training budget:** "night 1" (~50M tokens, ~5–6 h on the A4500) per
   cell is a floor. Want longer runs for the cells that pass the hard gates,
   or cap everything at night-1 for the first pass?
4. **Prompts for tests 1–3:** pull the *actual* prompts from
   `scripts/reasoning.py`, `scripts/decision_controller.py`, and
   `scripts/capability_registry.py` (so the test is faithful to the live
   gate), or use a synthetic fixed prompt set (more reproducible, less
   faithful)? I lean faithful + version-pinned.
