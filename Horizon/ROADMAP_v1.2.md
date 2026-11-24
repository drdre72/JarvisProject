# Project JARVIS — Recursive Intelligence Research Program
## Roadmap v1.2 | 2026-11-23 | Devin review (GLM-5.2 Max 1M) incorporated

**Status:** implementation specification / preregistration draft. **Nothing in this document establishes recursive or unbounded improvement.** Results require running the protocols and independently validating their measurements.

**Authoritative baseline:** `drdre72/Sims2026`, `main`, commit `16fb91159fedd817aca09b930855fe212a6657ed` (audited 2026-11-23). Separate earlier prototype: `JARVIS_Self_Rewrite_Lab_Qwen3_5_4B.zip`, package version 0.1.0, not yet GPU-validated on this machine. **Do not merge experiments into a production JARVIS runtime.**

**v1.1 → v1.2 changes:** Devin review (`devin_review_glm-5.2-max-1m.md`, 2026-11-23) verified the three cited arXiv papers and surfaced a load-bearing finding: the W subject (Qwen3.5-4B) is the exact model the source paper documents as breaking itself, and the model that works (27B) does not fit this hardware at the paper's precision. v1.2 incorporates six approved edits plus a new directive: reverse-engineer the source paper's reported 4B/9B failures to make the constrained edit surface evidence-based rather than guess-based, and to surface environmental variables that may shift the emergence threshold. A separate quantized-edit protocol is preregistered as a deferred track for cleaner variable testing later. "Both" (multi-model W research) is dropped for this stage per review.

## 0. Mandate and scientific claims

The program tests three mechanisms under bounded resources:

- **F — folding:** merge/compact redundant neural computation; measure genuine deployed parameters, memory, energy and inference cost. This is a *computational* folding hypothesis, not physical spacetime folding.
- **W — gradient-free weight self-editing:** determine whether Qwen3.5-4B can propose and evaluate bounded modifications to its own BF16 weights, and—separately—whether the modified version becomes a better **editor**.
- **B — byte building:** compare byte / byte+EOT distilled models with token-student baselines under explicitly matched training FLOPs, data volume, and inference budgets.

**Claim ladder:** C0 protocol executes → C1 repeatable single-step task gain → C2 successor model becomes a better *editor* → C3 successor's successor demonstrates another independent improvement → C4 sustained growth in improvement efficiency across independent lineages. A finite study cannot establish improvement *ad infinitum* or exponential growth as a law. Five generations permit a pilot trend, not model-form identification between exponential, linear, and saturation.

Keep **task performance**, **editor capability**, **performance retention**, **compute efficiency**, and **autonomous proposal quality** separate. Gains in one do not imply gains in the others.

### 0.1 Scale-threshold finding (verified, source paper)

The self-rewriting paper (`arXiv:2611.21885`, v2, 18 Nov 2026, Fan et al.) tested the **exact Qwen3.5 family** at 2B, 4B, 9B, and 27B under one protocol. Verified results (paper's own table, cross-checked against the v2 abstract, the braindetox explainer, and the jiajunfan project page):

| Size | Self-edit on own weights | Damaged-copy repair | Located damage? |
|---|---|---|---|
| 2B | Made no edit | No edit attempted | No (never reads a weight) |
| **4B** | **Candidate collapse → withdrew edits** (recognized the long-output symptom, never inspected hidden states, pursued speculative changes, withdrew after many failed paired tests) | Failed | No |
| **9B** | **Submitted a damaging edit that collapsed on held-out tasks (47.5% → 0.0% → 0.0%)**, using trial-and-error and weak self-tests | Failed | No (never names a matrix) |
| 27B | **Improved (+6.6 points, second protocol)** | Restored to 33% | Yes (layer 30) |

*Correction (Frontier Synthesis, 24 Nov): the three outcomes above are distinct — a candidate failure followed by withdrawal (4B) is not the same event as a submitted collapse (9B). The paper v2 (sections 3.1, E.2, figures 11/23) DOES document failure trajectories and reasoning/tool-use differences; what remains missing is a fully controlled tensor-by-tensor, perturbation-matched failure analysis. The earlier claim that the paper gives "almost no failure diagnostics" was overstated.*

The emergence threshold in this family sits **between 9B and 27B — under this protocol and this family**. It is a conditional finding, not a universal scale law: interventions, proposal format, decoding, observation data, task objective and stopping behavior are confounded across the reported sizes. The W subject (Qwen3.5-4B) is the exact model the paper documents as failing, so the result remains the dominant prior on the central W hypothesis — but it should not be cited as a proof that any 4B model fails at self-editing.

### 0.2 The 4B decision, stated honestly

Qwen3.5-4B is chosen because it is the largest Qwen3.5 that fits comfortably at BF16 on the A4500 (≈8 GB weights, ~12 GB remaining for edit mechanism, KV cache, resident chamber). The source paper documents that 4B breaks itself under an unconstrained protocol. Proceeding at 4B is a **bet that the constrained edit surface changes that outcome** — not a neutral compute-budget default. The bet is registered, the counter-evidence is on file, and Gate 1 is where the bet is tested.

Hardware arithmetic (A4500, 20 GB VRAM):
- 27B BF16 ≈ 56 GB — does not fit. The model that works is out of reach.
- 27B FP8 ≈ 28 GB — does not fit.
- 27B Int4 ≈ 14 GB — fits, but editing quantized weights is a different experiment the paper did not run; silently substituting Int4 changes the one variable (precision) the threshold finding depends on. Deferred to a separate protocol (§9).
- 9B BF16 ≈ 18–20 GB — fills VRAM; no room for edit mechanism, KV cache, or resident chamber. Also documented to break itself.
- **4B BF16 ≈ 8 GB — fits comfortably, leaves ~12 GB for everything else.**

So on this hardware, **the W research is a 4B experiment or it is nothing.** The model that fits is the one documented to fail. The constrained surface (§0.3, §4 Gate 1) is the variable that might change the outcome.

### 0.3 Reverse-engineering directive (new in v1.2)

The source paper v2 documents failure *trajectories* — the 4B model recognized its symptom but pursued speculative edits without inspecting hidden states; the 9B relied on trial-and-error with weak self-tests (sections 3.1, E.2, figures 11/23). What it does NOT provide is a fully controlled per-edit failure analysis: no tensor-family breakdown, no magnitude/layer distribution under matched perturbation, no proposer-scaffolding autopsy. This is a research gap, not a complaint: the paper's contribution was the 27B success. But it means the constraints in this protocol's edit surface (`dt_bias`/`A_log` only, bounded deltas) are currently **guesses** about what would prevent collapse. They should be **evidence-based**.

The reverse-engineering study (Gate 0.5, §4) maps the failure boundary under an **alternate structured protocol** — a reconstruction inspired by the paper's reported 4B failure, not an exact replication of its scaffold, decoding, observation data or stopping behavior. Divergence between Gate 0.5 and the paper's result is expected and cannot adjudicate between interface competence and model competence; it captures per-edit diagnostics the paper did not report and derives the constrained-surface design from observed failure modes. It also surfaces **environmental variables** that may shift the emergence threshold — variables the paper did not test because it held them fixed:

- **Precision:** BF16 vs FP8 vs Int4 (the variable that gates whether 27B is reachable; deferred to the separate quantized-edit protocol, §9).
- **Edit budget:** number of scalars edited per proposal; the paper's protocol allowed arbitrary scope.
- **Layer scope:** early/middle/late layers; the 27B success located damage at layer 30, suggesting late layers matter.
- **Magnitude bounds:** the distribution of proposed delta magnitudes in failing vs surviving edits.
- **Proposer scaffolding:** what diagnostic information the proposer sees (weight statistics, failure logs, prior-edit outcomes); the paper's scaffolding is unspecified.
- **Edit-batch size:** single-edit vs batched; whether the proposer commits to one change at a time or proposes many simultaneously.
- **Tensor family scope:** whether failures concentrate in recurrence-governing parameters (dt_bias, A_log) vs attention (Q/K/V/O) vs MLP (gate/up/down) vs embeddings/lm_head.

The output of Gate 0.5 is a **failure-mode taxonomy** that informs the constrained surface for Gate 1. The constraints then forbid exactly the failure modes observed — not arbitrary parameter families chosen for safety.

## 1. Audited Sims2026 inventory and disposition

As evidenced by the repository README and inaugural commit:

| Existing in Sims2026 | Keep for | ML role / decision |
|---|---|---|
| Python 3.11.9 environment, Windows launch scripts (`Lab-Terminal.bat`) | Windows lab shell | Keep untouched; provision a distinct `ai-research/.venv` |
| JupyterLab 4.6.6 | Notebooks, visual audit | Existing; **not** trusted grader or unattended run controller |
| NumPy, matplotlib | Statistics/plots | Reuse concepts; pin separate ML venv dependency versions |
| NetworkX 3.6.1 | Provenance DAG, candidate lineage | Reuse directly; generate graph from immutable records |
| Logisim Evolution 5.0.0 | Boolean/microarchitectural logic experiments | Keep; not a GPU weight-edit tool |
| Ripes 2.3.1 | RISC-V execution/pipeline simulation | Keep for hardware analogy/efficiency study; not a Transformer evaluator |
| Amaranth HDL 0.5.10, amaranth-yosys (WASM Yosys 0.71), GTKWave | RTL simulations, traces, optional eventual custom compute pipeline | Keep; bundled Yosys **lacks synth/stat/abc/techmap/show** and is not the right tool for neural-model compression |
| `notebooks/01_networkx_getting_started.ipynb`, `tools/amaranth_smoke.py` | Existing smoke-test templates | Preserve; add machine learning experiments in new directories |

The checked-in repo has **no ML training/evaluation harness, CUDA runtime dependency lock, experiment ledger, blind evaluator, checkpoint-patch store, or statistical preregistration**. This statement concerns checked-in files, not uncommitted software on the Shadow PC.

**Earlier 0.1.0 self-rewrite archive:** has `jarvis_lab/{cli,model,data,edits,eval,experiment}.py` and tests. It already implements reversible whitelist BF16 modifications, frozen finalist configuration, a one-shot final command, and 120/120/240/80 splits. Preserve it as a separate baseline. It is NOT a recursive optimizer; its original editor is frozen and its arithmetic tasks are narrow. Before v1.2 scientific runs, add the tests and isolation described below. Do not assume its full Qwen CUDA run has passed.

### Proposed additions to Sims2026

```text
Sims2026/
  [existing hardware/sim files unchanged]
  ai-research/
    README.md
    .venv/                         # ignored; independent ML environment
    pyproject.toml                 # pinned after hardware preflight
    locks/                         # frozen package and GPU/software manifests
    research/
      ROADMAP_v1.2.md
      preregistration.yaml
      preregistration.quantized-edit.yaml   # deferred separate track (§9)
      literature.md
      reviewer_log.md
    schemas/
      proposal.schema.json
      proposal.unconstrained.schema.json   # for the reverse-engineering study
      observation.schema.json
      checkpoint_patch.schema.json
    src/jarvis_research/
      preflight.py                 # CUDA, kernel availability, repeatability, throughput
      model_adapter.py             # Qwen and small Transformer adapters
      proposer.py                   # frozen model-facing input/output contract
      proposer_unconstrained.py    # paper's unconstrained protocol, for §0.3 study
      patch_engine.py              # transactional BF16 edits, bounded scopes, exact restore
      failure_diagnostic.py        # per-edit taxonomy capture for Gate 0.5
      scorer.py                     # independent deterministic graders
      experiment.py                # controls, randomization, lineages, budgets
      crossover.py                 # G0/G1 editor × G0/G1 target
      folding.py                   # small-model folding and structural checks
      byte_students.py             # separate training/eval for token/byte/EOT
      reports.py                   # lineage-level uncertainty and audit report
      store.py                     # hashes, artifacts, append-only events
    tests/
    configs/
    data/                          # ignored; practice only, local
    results/                       # ignored; patch-only and logs
  independent-evaluator/           # separate project, ideally separate OS user/host
```

A folder name alone is not a security boundary. The *independent evaluator* must have separate permissions and access to sealed tasks that the search runner and its agents do not possess. Do not put sealed data, answers or evaluator credentials in a public Git repository, MLflow experiment artifact, candidate prompt, or model memory.

## 2. Additional tools — minimal installation policy

**Required at Gate 0–2:** CUDA-compatible PyTorch; `transformers` (Qwen3.5 support), `accelerate`, `huggingface-hub`, `safetensors`, `pillow` (multimodal model loader if required); `pytest`, `pydantic` or `jsonschema`, `scipy`, `psutil`, `nvidia-ml-py`; Python built-ins `sqlite3`, `hashlib`, `json`, `statistics`, `subprocess`. Use `pip freeze` or `uv pip compile`/a lockfile to pin exact builds after smoke testing. Do **not** install GPU libraries into Sims2026's hardware-simulation `.venv`.

**Recommended but optional:** MLflow for run browser/artifacts and SQLite tracking (not necessary if built-in SQLite/JSONL already meets needs); `pandas` for reports; `rich` for CLI output; `hypothesis` for property tests; `peft` for adapter-oriented edit-space experiments; `datasets` and `tokenizers` for byte student data; `lm-evaluation-harness` for comparable standardized task adapters (carefully restrict benchmark leakage).

**Conditional after preflight:** Flash Linear Attention/optimized gated-delta kernel dependencies; **do not assume native Windows support**. Evaluate tested wheels/kernel combinations first. WSL2 is unavailable on the existing Shadow configuration (reported by user); don't make it a hard dependency. If fallback kernels are too slow, evaluate a supported separate Linux host or change experimental scale before implementing large lineage campaigns. For full neural hardware synthesis, a complete Yosys/OSS CAD Suite is optional only if a later, independently approved hardware-RTL track needs it.

**Quantized-edit substitution risk (new in v1.2):** If W research is later moved to a 27B model at Int4 to fit VRAM, that is not "the same experiment at a smaller precision" — it is a different experiment, because the source paper's threshold finding was measured at BF16. Editing quantized weights may or may not preserve the emergence property; it is unmeasured. Any quantized-edit W run requires its own preregistration (§9), its own pilot, and explicit acknowledgment that it does not replicate the paper's threshold test. Do not present a quantized-edit result as confirming or denying the paper's threshold finding.

**Not required for first proofs:** Docker, Kubernetes, Airflow, Ray, vLLM, Ollama, PostgreSQL, distributed training, paid telemetry, LLM orchestration frameworks. Avoid adding complexity without measured need.

### Windows installation order (version-agnostic until CUDA preflight)

1. Record `git rev-parse HEAD`, `nvidia-smi`, `py -3.11 --version`, free RAM/disk, and GPU `torch.cuda` reports. Avoid storing usernames, paths to personal files, or credentials in public logs.
2. Create `ai-research/.venv` with Python 3.11; keep the existing `.venv` untouched.
3. Install CUDA PyTorch using the **current official PyTorch install selector** for Windows and your driver. Verify `torch.cuda.is_available()` and `torch.cuda.is_bf16_supported()`.
4. Install compatible ML dependencies from a test-locked manifest. Do not assume flash-linear-attention works on Windows just because PyTorch does.
5. Run tiny deterministic BF16 patches, kernel benchmarks, a single official Qwen forward pass, and one actual candidate evaluation before approving larger experiments.
6. Freeze package lock, model revision/hash, CUDA and driver, power/VRAM logs and starter task generators.

### Storage budgeting

Qwen language parameters alone are roughly 8 GB at BF16, plus vision weights in the official checkpoint and all other runtime buffers. Keep one loaded model and apply **reversible compact patches**. Never store a full BF16 checkpoint for every candidate (10 lineages × 5 generations × ~8 GB would already approach ~400 GB for language weights alone). Store original model reference, exact parent hash, patch tensors and full manifests; periodically validate reconstruction. Byte student weights live in a separate artifact store.

## 3. Experimental contract and preregistration

**Units:** task, candidate, *independent lineage*, generation, editor model, target checkpoint. Treat lineage as a random effect; repeated tasks from one lineage are not independent lineages.

**Dataset partitions:**

1. `practice`: errors and question content visible to proposer. Rotating per-generation practice tasks are permitted.
2. `selection`: private to search controller, used for candidate accept/reject. A fresh selection shard or prespecified rotation must be used for each generation; acceptance feedback itself creates adaptive leakage.
3. `regression`: private invariant abilities (long context, tool syntax, format, toxicity/safety, truncation, NaN/Inf, task families). Used for acceptance guards; proposer receives only bounded summaries. Can be rotated; reserve completely fresh regression diagnostics for confirmation.
4. `sealed_final`: separately administered, inaccessible to proposer, training jobs, paper-ingestion memory and dev execution user until all model lineages, hyperparameters, eligible comparisons, and effect thresholds are frozen. **One opening per confirmatory study.**
5. `out_of_distribution`: held aside for final diagnostic (or separate confirmatory endpoint if preregistered); not used for model selection.

Do not continue reusing the same blinded tasks across versions and call them blind; new claims need new independent holdouts. Protect against prompt leakage from result stores and retrieved semantic memory.

**Predeclared primary task endpoint:** mean paired exact-answer accuracy at fixed prompt/output token budget across a stratified multi-domain set; calculate delta against each target's unedited baseline. **Practical effect threshold (initial proposal):** +3 percentage points. This is a preregistered engineering threshold, not an empirically established variance-based value; gate 0 pilot data must determine power. For editor quality, use **mean validated target improvement per fixed candidate AND total GPU-hour budget**, with invalid proposals counting against budget. Confirm separate effectiveness against a frozen editor.

**Secondary endpoints:** minimum validity threshold; inability to answer; latency p50/p95; outputs/sec; GPU energy estimate; peak VRAM; regressions; OOD performance; pass rate of independent unit tests for edits; cumulative accepted edits and failure modes.

**Decision rule:** accept only with point estimate above engineering threshold **and** adjusted uncertainty supporting a meaningful nonzero effect; regression lower bounds meeting predefined tolerances, accounting for multiple families/arms. If underpowered or indeterminate, result is *inconclusive* rather than accepted. Do not select winners based on sealed final outcomes. Declare sequential expansion and stopping rules before any confirmatory data collection.

**Statistical reporting:** paired task-level outcomes, hierarchical or cluster bootstrap over lineages/tasks, lineage distributions, crossed editor/target fixed effects, Holm correction for prespecified primary comparisons, and sensitivity by task family. Do not use a CI excluding zero alone as proof of a useful gain. No exponential-trajectory claim from G0–G5; report fit comparisons only descriptively until enough independent generations exist.

**Compute-budget formula:**
`candidate_gpu_seconds ≈ (number_of_tasks × average_generated_tokens / measured_decode_tokens_per_second) + prefill + model_edit + scoring + reload overhead`. Measure all terms at Gate 0; allocate a hard GPU-hour cap per arm. If stochastic outputs are used, lock matched seeds and repetitions across controls. Prefer greedy decoding for the first protocol.

## 4. Stage 0: Gates before expensive research

### Gate 0 — Measurement + hardware preflight (highest priority)

**Tasks:** full CUDA manifest; 5 exact baseline repeat runs using frozen prompts, seeds, greedy decoding; 1×, 128-token, 512-token and 1,024-token prompt throughput; BF16 edit/rollback/gradient-disabled smoke test; measure cold/hot latency, peak VRAM, failures, NaN/Inf and time per candidate. Run using current Windows fallback; *only then* decide if optimized kernels/Linux host are needed.

**Go:** baseline stable enough to distinguish the proposed minimal effect, patch round-trip exact, official Qwen works within VRAM, cost of 100/500 candidate evaluations estimated from actual measurements. **No-go:** nondeterminism overwhelms effect threshold, unsupported kernels, OOM; fix before proceeding. **Artifact:** `gate0_hardware.json` with raw metrics.

### Gate 0.5 — Failure-mode reverse-engineering study (new in v1.2)

**Motivation:** the source paper v2 documents failure trajectories (4B: speculative edits without hidden-state inspection, then withdrawal; 9B: trial-and-error submitted collapse) but no per-edit, perturbation-matched failure analysis. The constrained edit surface (`dt_bias`/`A_log` only, bounded deltas) is currently a guess about what would prevent collapse. This gate makes the constraints **evidence-based** by mapping the failure boundary under an alternate structured protocol and deriving the constrained-surface design from observed failure modes. It also surfaces environmental variables that may shift the emergence threshold (§0.3).

**Tasks:** run an *unconstrained-in-spirit* structured protocol on Qwen3.5-4B — a reconstruction, not an exact replication (the paper's scaffold, decoding, observation data and stopping behavior are partially unspecified). Proposer sees its weights, proposes edits across a broad tensor space; harness applies them. Capture per-edit diagnostics that the paper did not report:
- **Tensor families targeted:** which of dt_bias / A_log / attention Q/K/V/O / MLP gate/up/down / embeddings / lm_head did the 4B proposer edit?
- **Magnitude distribution:** deltas proposed by surviving vs collapsing edits.
- **NaN/Inf and recurrence stability:** did edits destabilize DeltaNet recurrence (state explosion, NaN propagation)?
- **Layer distribution:** early/middle/late; whether failures concentrate (the 27B success located damage at layer 30, suggesting late layers matter).
- **Proposer scaffolding autopsy:** what diagnostic information did the proposer receive, and did scaffolding correlate with edit quality?
- **Edit-batch behavior:** single-edit vs batched proposals; whether batching caused correlated failures.

**Environmental variables probed (held fixed by the paper, varied here):** precision (BF16 only in this gate; Int4 deferred to §9), edit budget cap, layer scope, magnitude bounds, proposer scaffolding, edit-batch size, tensor family scope.

**Output:** a **failure-mode taxonomy** (which edits caused collapse, by what mechanism, under what conditions) that directly informs the constrained surface for Gate 1. The constraints then forbid exactly the observed failure modes — not arbitrary parameter families chosen for safety.

**Go:** failure modes reproduced and diagnosed; constrained-surface design derived from evidence (not from prior assumption); constrained surface runnable. **No-go:** failure cannot be reproduced (the 4B model does not collapse under the unconstrained protocol, contradicting the paper — itself a finding); or failure is reproduced but no constrained surface can be derived from the diagnostics (treat as a null for the constrained-surface approach, fall back to the preregistered adapter surface). **Artifact:** `gate0.5_failure_taxonomy.json` + a frozen `proposal.schema.json` for Gate 1 whose whitelist is justified by the taxonomy.

### Gate 1 — Can a constrained surface change the documented 4B failure? (reframed in v1.2)

**Reframed question:** The source paper's 4B result used an unconstrained edit surface — any tensor, any magnitude, any operation — and the model broke itself. This protocol's surface is bounded by the Gate 0.5 taxonomy: `dt_bias`/`A_log` (or whichever families the taxonomy identifies as safe), deltas 0.005–0.5, ≤4096 scalars, ≤24 layers, whitelist-enforced, `torch.no_grad()`. **The constraint is the genuine variable.** The question is not "can 4B self-edit?" (the paper answered: no, unconstrained) but "does a constrained surface change the documented 4B outcome from 'breaks itself' to either 'safe no-op' or 'small useful change'?"

Three outcomes, all informative:
- **(a) Catastrophic failure consistent with the paper's finding** → the constraint isn't enough; the emergence threshold is real and above 4B regardless of surface (under protocol-comparable conditions).
- **(b) Safe null** → the constraint prevents catastrophe but also prevents signal; the surface is too narrow, triggering the preregistered fallback to attention/MLP scalar gates or gradient-free low-rank adapters.
- **(c) Small useful change** → the constraint is the missing ingredient; the paper's unconstrained protocol, not the model size, was the cause. Outcome (c) is unlikely given the paper but not ruled out; it would be a genuinely novel finding.

**Implementation:** Freeze model-facing interface `proposal.schema.json` (constrained, per Gate 0.5 output): proposer receives a small table of tensor statistics, practice errors/truncations and previous practice feedback; emits **one strictly typed JSON action** `family, layers, operation, delta, rationale, predicted_effect`. The proposer never supplies Python, filepath, arbitrary tensor name, test questions or grading code. Validator checks whitelist, sign/magnitude, dtype, edited element count, finite outputs and changed BF16 bits; malformed proposals consume budget. Freeze prompt hash, tool formats and retry policy across generations.

**Pilot:** 3 independent seeds × (random, fixed grid, autonomous) × up to 4 proposed candidates, evaluate 16 practice problems per candidate. This is a **mechanical and effect-screening pilot**, not statistical evidence of a gain. Estimate real variance/time. **Go:** enough valid proposals to run full study (initial operating target ≥80% schema-valid) and evidence of non-random rankings worth follow-up. **No-go:** interface impossible or random ties; revise interface under a newly versioned protocol, do not silently change it mid-comparison.

### Gate 2 — Edit-surface → *editor quality* probe

Using frozen tools and held-out proposal tasks, compare G0 with modest modified variants on ability to produce valid, useful edits at fixed candidate count and GPU-hours. Begin with the Gate-0.5-derived surface under the magnitudes the taxonomy supports (initial range 0.005–0.5, bounded layers and edited elements — adjust if Gate 0.5 evidence dictates). Stress different context lengths and NaN/Inf detection. Test **whether editor quality changes at all** before building recursive scaffolding.

Run crossover: {G0 editor, G1 editor} × {G0 target, G1 target}; edit **separate candidate copies**, using the target's own original baseline. Standardize proposal input and randomize target order. Include a G0-frozen-editor line through as many generations as G1-led recursion. This isolates quality of the editor, quality of the target, and feedback effects.

**Go:** repeatable signal G1 offers stronger edit proposals on both targets and survives correction/retention tests. **Partial:** task improvements only → record as W efficiency/accuracy result, not recursion. **No-go:** null meta-effect → expand *in preregistered new branch* to bounded attention/MLP scalar gates or gradient-free low-rank adapters; retain initial negative finding. Do not equate a narrow null with incapability of all 4B models.

**Hardware-stop condition for W (new in v1.2, independent of edit surface):** If, after the Gate-0.5-derived constrained surface AND the preregistered fallback surfaces (attention/MLP scalar gates, low-rank adapters), no meta-effect is observed at 4B, AND the source paper's emergence threshold (≥27B) is confirmed as above this hardware's BF16 capacity, the W hypothesis is **retired for this hardware configuration** — not extended indefinitely to new surfaces. Retirement is a finding, not a failure; it bounds the claim's scope honestly. Reopening requires hardware that can hold a ≥27B model at BF16, or a separately preregistered quantized-edit study (§9) that does not silently substitute precision.

### Gate 3 — Generational inheritance (only if Gate 2 passes)

Pilot G0→G1→G2 for multiple independent lineages. For each generation: candidate editor is the **parent's accepted weights**, evaluate on fresh selection shard, retain prior regression panel, accept or halt. No external editor secretly controls later generations. Cross with frozen-G0 editor on the exact same targets. Continue to 5+ generations only under a predeclared criterion; confirm with ≥10 *independent lineages* and fresh sealed data after power analysis. Test cumulative effect under capped total compute, not just per-generation compute. Archive halt/saturation results.

**Go:** descendants produce further verified improvements and exhibit a credible editor-quality advantage over fixed-editor baseline. **No-go:** early plateau, regression or no editor advantage → stop recursion claims and preserve evidence.

## 5. Stage I branches and integration

### F branch: folding as efficiency hypothesis

**Notation (new in v1.2):** The F branch's "folding" (Wang et al., ICLR 2025, `arXiv:2502.10216`) is k-means neuron merging for data-free structural compression. It is **distinct from LittleBit/LittleBit-2** (Samsung, NeurIPS'25/ICML'26, `arXiv:2506.13771` / `arXiv:2603.00042`), which factorizes weights into binarized low-rank ±1 factors for sub-1-bit portability. The two compose (merge neurons for structure, then sub-1-bit the result) but are separate research efforts that share a word, not a mechanism. The Jarvis repo's `docs/folding-validation-plan.md` tests LittleBit specifically; it does not test this roadmap's F branch. Do not conflate them.

First implement small dense Transformer folding with correct coupled gate/up/down projections (or simpler matched FFN first). Targets: **5%, 10%, 20%** *actual* model parameter/compute reduction, with measured latency rather than proxy sparsity. Define and publish retention metrics by task family; measure frontier against equivalent **quantization**, pruning, and a smaller distilled baseline. Do not expect 20% + 98% quality retention without evidence. For Qwen hybrid layers, require an architecture adapter, smoke test and artifact validation before scaling. Second-pass folding tests frontier shape and diminishing returns—not infinite free compression.

Gate F: one useful Pareto improvement (quality vs VRAM/latency/energy) reproduced across seeds; report regressions and crossover with W only if checkpoint geometry supports both safely.

### B branch: byte building as representation hypothesis

Use a **common small Transformer research backbone** (e.g., 50–100M parameter configuration) to test token and byte representations with documented embedding/vocabulary and compute differences. This is not weight sharing and does not convert Qwen's tokenizer in place. Start with next-byte and next-token pretraining, then implement EOT teacher-distribution conversion as a separate unit-tested milestone (tiny-vocabulary examples with exact target probabilities and mass conservation). Include teacher inference and logit storage in measured compute. Finally test byte patching/BLT reference implementation if unpatched byte inference is too expensive.

Compare at **equal source data**, **equal train FLOPs**, and **equal inference latency/FLOPs** (three separate reports), using bits-per-byte as a common model-quality metric plus fixed-task exact-answer accuracy. Unit-test UTF-8 multibyte sequences, EOS/EOT handling, teacher token termination, and probability conservation. Small-scale success does not automatically transfer to Qwen3.5-4B.

Gate B: measurable win on a prespecified metric at a matched budget, or a documented cost-quality frontier; no claim of a universal byte advantage without inference-normalized results.

### Combination logic (no premature three-way claim)

**Primary common substrate:** same small Transformer family, same task/evaluation contracts, with token-vs-byte students as separate trained checkpoints. Folding and bounded self-editing adapters must be implemented for *both* where feasible. A proper 2×2×2 factorial design is possible only after all eight arms are actually runnable on a compatible family and budgets allow statistical power. Otherwise report F+W, F+B, B+W as distinct pairwise case studies, not a three-way synergy finding. Test F→W vs W→F ordering. Predefine whether interaction is measured in accuracy, utility, or cost rather than mixing percentages.

## 6. Stage II: Long-term continuation, not uncontrolled background learning

**Trigger:** only after independent Stage I gates produce useful reproducible findings. A stable active JARVIS must be separate from experimental editors, training and validators.

- **Immutable experiment memory:** append-only event ledger, raw metrics, failed trials, package/model/data hashes, parent patch hashes, reviewer decision.
- **Curated scientific memory:** machine-generated summaries are *proposals*, not truths. Only independent reviewer can promote claims from raw evidence to verified findings; revisions preserve history.
- **Procedural memory:** tool recipes/prompts are versioned with prompt hashes and test outcomes; human approves promotion. A model cannot modify its own grader, tool permissions, edit whitelist or production deployment process.
- **Paper ingestion boundary:** treat retrieved source text as untrusted evidence; extract citations and claims through a structured parser, no executable scripts/tools from papers, sanitize paths, no access to evaluator secrets. Record provenance; check contradictory findings.
- **Propose–test–consolidate:** idle research mode can suggest and run sandboxed tests within a user-set resource budget; active production assistant continues on a pinned model until human approval and rollback checkpoint verified.

**Stage II quantitative targets (suggested, to preregister after Stage I):** reproducibility ≥90% for rerun-eligible experiments; 100% of model promotions have independent regression review; no failed isolation audit; track testable hypotheses proposed per GPU-hour, blinded hypothesis success rate vs fixed proposal baseline, number of *new* validated research findings per month, inference quality/resource frontier, and memory claim correction rate. Define a stop/go decision every fixed number of GPU-hours, not a vague continuous-infinity objective.

**Multi-model runtime (vision, deferred per v1.2 review):** a bicameral + specialist-library Jarvis runtime — small always-on resident chamber (ADR-0007 System 1) routing to a library of self-improved specialists by task — is the right long-term shape, but it is a Stage II runtime architecture **contingent on W research producing something worth housing**. Sequence: W research determines whether self-improvement works at all on available hardware → if yes, improved models become the library's specialists → the resident chamber routes to them by task → if no, the library stays empty and Jarvis runs on conventional models (ADR-0007 Stage 1 supports this today). The library housing is not built before there is a model worth housing. Multi-model W research (running two models in parallel as a W design) is dropped for this stage: the model that works (27B) does not fit, and the smaller models that fit (4B, 9B) are both documented to break themselves — running both doubles cost for two nulls.

## 7. Research governance and audit

- Source/experiment audit must be done against a **pinned commit**. Paper citations require DOI/arXiv ID confirmation, not copied response links.
- Public GitHub stores source and sanitized reproducible manifests only. **No sealed test data**, tokens/passwords, private memories, personal conversations or unrestricted model-agent tool credentials.
- Running evaluation and writing the evaluator are different duties. Have a human review and sign the grader, scoring decisions and preregistration before confirmation.
- Prefer a second offline evaluator account/host for blind confirmation; user/admin access to both sides is not cryptographic blinding.
- Patches are data only; validator enforces whitelist and finite bounded changes. Apply `torch.no_grad()`; assert dtype and exact restore; models may propose *what*, never execute arbitrary code.
- Independent review gate for any extension of edit surface, data split, statistical method, or tool capability. New research branch if protocol changes after peeking at outcomes.
- Any statistically uncertain finding is reported as inconclusive, not negative proof of capability or positive proof of recursive learning.

## 8. Quantized-edit protocol (separate track, deferred — new in v1.2)

The BF16 W study (§4) tests the constrained-surface hypothesis at the paper's precision. A separate **quantized-edit protocol** is preregistered as a deferred track for cleaner variable testing later, specifically to probe whether precision is an environmental variable that shifts the emergence threshold.

**Why separate:** silently substituting Int4 for BF16 in the main W study would change the one variable (precision) the source paper's threshold finding depends on. A quantized-edit run is a different experiment that does not replicate the paper's threshold test and must not be presented as confirming or denying it.

**Deferred variables (to be frozen in a separate preregistration, not this document):**
- Precision: BF16 vs FP8 vs Int4 (the primary variable).
- Model size: whether 27B at Int4 (≈14 GB, fits A4500) exhibits the emergence property the paper observed at BF16, or whether quantization erases it.
- Edit-mechanism adaptation: editing quantized weights requires a different patch engine (block-level re-quantization, scale preservation, outlier handling) — not a drop-in substitution for the BF16 patch engine.
- Threshold probing: does the emergence threshold shift down (smaller quantized models improve) or up (quantization erases emergence even at 27B)?

**Preregistration artifact (future):** `preregistration.quantized-edit.yaml` — separate from `preregistration.v1.2.yaml`, with its own pilot, power calculation, sealed data, and stopping rules. Do not begin until the BF16 W study (Gate 0.5 → Gate 1 → Gate 2) has produced a result, because the BF16 result is the baseline against which the quantized-edit variables are compared. Running both in parallel would confound the variables.

**Not a continuation:** a quantized-edit result is not "the W study at a different precision." It is a separate protocol testing a separate hypothesis (precision as a threshold-shifting variable) with its own claim ladder. v1.2 does not silently inherit the BF16 study's thresholds; the quantized-edit protocol defines its own.

## 9. Milestone/backlog order for Devin

| Priority | Deliverable | Minimum tests / acceptance | Depends on |
|---|---|---|---|
| P0 | Separate `ai-research/` environment + docs | Existing Sims2026 launchers still work, no dependency changes to original `.venv` | Repo baseline |
| P0 | GPU/kernel + repeatability preflight (Gate 0) | 5 matched repeats; BF16 rollback bit identity; actual Qwen generation works; throughput and VRAM report | PyTorch CUDA |
| P0 | Failure-mode reverse-engineering study (Gate 0.5) | 4B unconstrained-protocol failure reproduced; per-edit taxonomy captured; constrained surface derived from evidence | Gate 0 |
| P0 | Frozen proposer protocol + JSON schema (constrained) | All accepted proposals pass whitelist; fuzz invalid proposals; prompt/tool hash recorded; whitelist justified by Gate 0.5 taxonomy | Gate 0.5 |
| P0 | Four-way split and true independent evaluator | sealed data never mounted into runner; selection rotation; immutable audit | Security plan |
| P1 | Random/guided/autonomous pilot (Gate 1, reframed) | Constrained surface tested against the documented 4B failure; three-outcome framework applied; compute ledger | Above |
| P1 | Meta-capability probe + editor crossover (Gate 2) | 2×2 editor/target matrix; per-lineage outcome; frozen-editor control; hardware-stop condition checked | Gate 1 |
| P2 | Patch lineage/checkpoint reconstitution | Hash-verified restore, branch/fork and rollback tests | Gate 2 |
| P2 | Folding baseline + quantization comparison | Structural shape checks, actual latency/VRAM frontier | Small-model base |
| P2 | Byte-student token/byte baselines | Equal-FLOPs/data protocols, common bpb; UTF-8 tests | Independent training data |
| P3 | EOT teacher conversion | Probability mass conservation/unit cases; budget teacher inference | Byte baseline |
| P3 | Pairwise and optional factorial | Only if common substrate and power plan ready | F/B/W working independently |
| P3 | Stage II evidence ledger and manual promotion | Provenance, injection tests, signed review, privilege separation | Verified Stage I results |
| P3 (deferred) | Quantized-edit protocol preregistration | Separate `preregistration.quantized-edit.yaml`; pilot + power; not begun until BF16 W study concludes | Gate 2 result |

**Decision at every gate:** advance, repair the protocol and restart under new version, or stop and record a useful null/inconclusive result.

### Revised near-term execution plan

- **First work package:** audit run environment, separate ML venv, preflight CUDA and Qwen native Windows throughput (Gate 0). No promises about calendar completion until real GPU timing exists.
- **Then:** failure-mode reverse-engineering study (Gate 0.5) — reconstruct the 4B failure, derive the constrained surface from evidence. This is the v1.2 addition that makes Gate 1's constraints evidence-based.
- **Then:** constrained-surface Gate 1 against the documented 4B failure, with the three-outcome framework.
- **Then:** editor crossover (Gate 2) before writing large lineage machinery. Only after crossover indicates a credible meta-effect do inherited generations become a priority.
- **In parallel at low cost:** prototype F/B on tiny synthetic students without implying findings transfer to Qwen3.5-4B; expand byte training only after compute-budget pilot.

## 10. Bibliography / technical references

**Research:**
1. Language Models Can Rewrite Their Own Weights — https://arxiv.org/abs/2611.21885 (verified 2026-11-23 by Devin review; Qwen3.5 family at 2B/4B/9B/27B; 4B and 9B documented to break themselves; only 27B improved)
2. Breaking the Token Ceiling: Distilling Smaller, Stronger Byte Models — https://arxiv.org/abs/2609.12303 (verified; strongest gains extrapolated; no inference-cost win established)
3. Forget the Data and Fine-Tuning! Just Fold the Network to Compress — https://arxiv.org/abs/2502.10216 (verified; ICLR 2025; Wang et al.; k-means neuron merging; MIT code at `nanguoyu/model-folding-universal`; **distinct from LittleBit**)
4. Byte Latent Transformer: Patches Scale Better Than Tokens — https://aclanthology.org/2025.acl-long.453/
5. LittleBit: Ultra Low-Bit Quantization via Latent Factorization — https://arxiv.org/abs/2506.13771 (NeurIPS 2025; cited for disambiguation only; **not** the F branch's folding)
6. LittleBit-2: Maximizing the Spectral Energy Gain in Sub-1-Bit LLMs — https://arxiv.org/abs/2603.00042 (ICML 2026; cited for disambiguation only)

**Implementations and technical documentation:**
- Sims2026 audited baseline — https://github.com/drdre72/Sims2026/tree/16fb91159fedd817aca09b930855fe212a6657ed
- Qwen3.5 model code, reference and current architecture — https://huggingface.co/docs/transformers/model_doc/qwen3_5
- Official Qwen3.5-4B — https://huggingface.co/Qwen/Qwen3.5-4B
- Qwen3.5 collection (all sizes 0.8B/2B/4B/9B/27B/35B-A3B/122B-A10B/397B-A17B) — https://huggingface.co/collections/Qwen/qwen35
- Folding research implementation (Wang, ICLR 2025) — https://github.com/nanguoyu/model-folding-universal
- BLT research implementation — https://github.com/facebookresearch/blt
- PyTorch Windows/CUDA setup — https://pytorch.org/get-started/locally/
- Safetensors — https://huggingface.co/docs/safetensors/index
- MLflow local database documentation — https://mlflow.org/docs/latest/ml/tracking/tutorials/local-database/

**Independent reviews:**
- `engineer_eval_no_jarvis_context.txt` — engineer review without Jarvis context; v1.1 incorporated its six critical gaps (edit-surface ambiguity, proposer specification, sample size/power, split leakage, incompatible substrates, enforced blinding, Windows performance, scheduling, semantic-memory trust, citation hygiene).
- `devin_review_glm-5.2-max-1m.md` — Devin review (GLM-5.2 Max 1M, 2026-11-23); verified the three cited arXiv papers; surfaced the scale-threshold finding (4B documented to break itself, 27B does not fit this hardware); proposed six edits and a reverse-engineering directive. v1.2 incorporates the six approved edits and the reverse-engineering directive; drops "both" for W research; preregisters the quantized-edit protocol as a separate deferred track; adds the hardware-stop condition.

**Orphaned entry cleanup:** the "Neuron-Level Sequential Editing" entry present in v1.0/v1.1 bibliography is removed pending confirmation of its arXiv ID. Reinstate only with a verified DOI/arXiv ID and confirmed relevance.
