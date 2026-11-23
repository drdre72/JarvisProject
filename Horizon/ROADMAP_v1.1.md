# Project JARVIS — Recursive Intelligence Research Program
## Roadmap v1.1 | 2026-11-23 | Independent Engineering Review 001 incorporated

**Status:** implementation specification / preregistration draft. **Nothing in this document establishes recursive or unbounded improvement.** Results require running the protocols and independently validating their measurements.

**Authoritative baseline:** `drdre72/Sims2026`, `main`, commit `16fb91159fedd817aca09b930855fe212a6657ed` (audited 2026-11-23). Separate earlier prototype: `JARVIS_Self_Rewrite_Lab_Qwen3_5_4B.zip`, package version 0.1.0, not yet GPU-validated on this machine. **Do not merge experiments into a production JARVIS runtime.**

## 0. Mandate and scientific claims

The program tests three mechanisms under bounded resources:

- **F — folding:** merge/compact redundant neural computation; measure genuine deployed parameters, memory, energy and inference cost. This is a *computational* folding hypothesis, not physical spacetime folding.
- **W — gradient-free weight self-editing:** determine whether Qwen3.5-4B can propose and evaluate bounded modifications to its own BF16 weights, and—separately—whether the modified version becomes a better **editor**.
- **B — byte building:** compare byte / byte+EOT distilled models with token-student baselines under explicitly matched training FLOPs, data volume, and inference budgets.

**Claim ladder:** C0 protocol executes → C1 repeatable single-step task gain → C2 successor model becomes a better *editor* → C3 successor's successor demonstrates another independent improvement → C4 sustained growth in improvement efficiency across independent lineages. A finite study cannot establish improvement *ad infinitum* or exponential growth as a law. Five generations permit a pilot trend, not model-form identification between exponential, linear, and saturation.

Keep **task performance**, **editor capability**, **performance retention**, **compute efficiency**, and **autonomous proposal quality** separate. Gains in one do not imply gains in the others.

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

**Earlier 0.1.0 self-rewrite archive:** has `jarvis_lab/{cli,model,data,edits,eval,experiment}.py` and tests. It already implements reversible whitelist BF16 modifications, frozen finalist configuration, a one-shot final command, and 120/120/240/80 splits. Preserve it as a separate baseline. It is NOT a recursive optimizer; its original editor is frozen and its arithmetic tasks are narrow. Before v1.1 scientific runs, add the tests and isolation described below. Do not assume its full Qwen CUDA run has passed.

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
      ROADMAP_v1.1.md
      preregistration.yaml
      literature.md
      reviewer_log.md
    schemas/
      proposal.schema.json
      observation.schema.json
      checkpoint_patch.schema.json
    src/jarvis_research/
      preflight.py                 # CUDA, kernel availability, repeatability, throughput
      model_adapter.py            # Qwen and small Transformer adapters
      proposer.py                 # frozen model-facing input/output contract
      patch_engine.py             # transactional BF16 edits, bounded scopes, exact restore
      scorer.py                   # independent deterministic graders
      experiment.py               # controls, randomization, lineages, budgets
      crossover.py                # G0/G1 editor × G0/G1 target
      folding.py                  # small-model folding and structural checks
      byte_students.py            # separate training/eval for token/byte/EOT
      reports.py                  # lineage-level uncertainty and audit report
      store.py                    # hashes, artifacts, append-only events
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

### Gate 1 — Can model proposals beat random and fixed search?

Freeze model-facing interface `proposal.schema.json`: proposer receives a small table of tensor statistics, practice errors/truncations and previous practice feedback; emits **one strictly typed JSON action** `family, layers, operation, delta, rationale, predicted_effect`. The proposer never supplies Python, filepath, arbitrary tensor name, test questions or grading code. Validator checks whitelist, sign/magnitude, dtype, edited element count, finite outputs and changed BF16 bits; malformed proposals consume budget. Freeze prompt hash, tool formats and retry policy across generations.

**Pilot:** 3 independent seeds × (random, fixed grid, autonomous) × up to 4 proposed candidates, evaluate 16 practice problems per candidate. This is a **mechanical and effect-screening pilot**, not statistical evidence of a gain. Estimate real variance/time. **Go:** enough valid proposals to run full study (initial operating target ≥80% schema-valid) and evidence of non-random rankings worth follow-up. **No-go:** interface impossible or random ties; revise interface under a newly versioned protocol, do not silently change it mid-comparison.

### Gate 2 — Edit-surface → *editor quality* probe

Using frozen tools and held-out proposal tasks, compare G0 with modest modified variants on ability to produce valid, useful edits at fixed candidate count and GPU-hours. Begin `dt_bias`/`A_log` under existing conservative magnitudes (0.005–0.5, bounded layers and edited elements). Stress different context lengths and NaN/Inf detection. Test **whether editor quality changes at all** before building recursive scaffolding.

Run crossover: {G0 editor, G1 editor} × {G0 target, G1 target}; edit **separate candidate copies**, using the target's own original baseline. Standardize proposal input and randomize target order. Include a G0-frozen-editor line through as many generations as G1-led recursion. This isolates quality of the editor, quality of the target, and feedback effects.

**Go:** repeatable signal G1 offers stronger edit proposals on both targets and survives correction/retention tests. **Partial:** task improvements only → record as W efficiency/accuracy result, not recursion. **No-go:** null meta-effect → expand *in preregistered new branch* to bounded attention/MLP scalar gates or gradient-free low-rank adapters; retain initial negative finding. Do not equate a narrow null with incapability of all 4B models.

### Gate 3 — Generational inheritance (only if Gate 2 passes)

Pilot G0→G1→G2 for multiple independent lineages. For each generation: candidate editor is the **parent's accepted weights**, evaluate on fresh selection shard, retain prior regression panel, accept or halt. No external editor secretly controls later generations. Cross with frozen-G0 editor on the exact same targets. Continue to 5+ generations only under a predeclared criterion; confirm with ≥10 *independent lineages* and fresh sealed data after power analysis. Test cumulative effect under capped total compute, not just per-generation compute. Archive halt/saturation results.

**Go:** descendants produce further verified improvements and exhibit a credible editor-quality advantage over fixed-editor baseline. **No-go:** early plateau, regression or no editor advantage → stop recursion claims and preserve evidence.

## 5. Stage I branches and integration

### F branch: folding as efficiency hypothesis

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

## 7. Research governance and audit

- Source/experiment audit must be done against a **pinned commit**. Paper citations require DOI/arXiv ID confirmation, not copied response links.
- Public GitHub stores source and sanitized reproducible manifests only. **No sealed test data**, tokens/passwords, private memories, personal conversations or unrestricted model-agent tool credentials.
- Running evaluation and writing the evaluator are different duties. Have a human review and sign the grader, scoring decisions and preregistration before confirmation.
- Prefer a second offline evaluator account/host for blind confirmation; user/admin access to both sides is not cryptographic blinding.
- Patches are data only; validator enforces whitelist and finite bounded changes. Apply `torch.no_grad()`; assert dtype and exact restore; models may propose *what*, never execute arbitrary code.
- Independent review gate for any extension of edit surface, data split, statistical method, or tool capability. New research branch if protocol changes after peeking at outcomes.
- Any statistically uncertain finding is reported as inconclusive, not negative proof of capability or positive proof of recursive learning.

## 8. Milestone/backlog order for Devin

| Priority | Deliverable | Minimum tests / acceptance | Depends on |
|---|---|---|---|
| P0 | Separate `ai-research/` environment + docs | Existing Sims2026 launchers still work, no dependency changes to original `.venv` | Repo baseline |
| P0 | GPU/kernel + repeatability preflight | 5 matched repeats; BF16 rollback bit identity; actual Qwen generation works; throughput and VRAM report | PyTorch CUDA |
| P0 | Frozen proposer protocol + JSON schema | All accepted proposals pass whitelist; fuzz invalid proposals; prompt/tool hash recorded | Existing 0.1.0 harness |
| P0 | Four-way split and true independent evaluator | sealed data never mounted into runner; selection rotation; immutable audit | Security plan |
| P1 | Random/guided/autonomous pilot | Candidate budget and invalid attempts fairly counted; compute ledger | Above |
| P1 | Meta-capability probe + editor crossover | 2×2 editor/target matrix; per-lineage outcome; frozen-editor control | Gate 1 |
| P2 | Patch lineage/checkpoint reconstitution | Hash-verified restore, branch/fork and rollback tests | Gate 2 |
| P2 | Folding baseline + quantization comparison | Structural shape checks, actual latency/VRAM frontier | Small-model base |
| P2 | Byte-student token/byte baselines | Equal-FLOPs/data protocols, common bpb; UTF-8 tests | Independent training data |
| P3 | EOT teacher conversion | Probability mass conservation/unit cases; budget teacher inference | Byte baseline |
| P3 | Pairwise and optional factorial | Only if common substrate and power plan ready | F/B/W working independently |
| P3 | Stage II evidence ledger and manual promotion | Provenance, injection tests, signed review, privilege separation | Verified Stage I results |

**Decision at every gate:** advance, repair the protocol and restart under new version, or stop and record a useful null/inconclusive result.

### Revised near-term execution plan

- **First 48 hours of actual engineering:** audit run environment, separate ML venv, preflight CUDA and Qwen native Windows throughput. No promises about calendar completion until real GPU timing exists.
- **Next work package:** replace narrow arithmetic-only editor evaluation with a fixed multi-domain proposer rubric, schema, random/guided comparisons, and independent score sets.
- **Then:** test editor crossover before writing large lineage machinery. Only after crossover indicates a credible meta-effect do inherited generations become a priority.
- **In parallel at low cost:** prototype F/B on tiny synthetic students without implying findings transfer to Qwen; expand byte training only after compute-budget pilot.

## 9. Bibliography / technical references

**Research:**
1. Language Models Can Rewrite Their Own Weights — https://arxiv.org/abs/2611.21885
2. Breaking the Token Ceiling: Distilling Smaller, Stronger Byte Models — https://arxiv.org/abs/2609.12303
3. Forget the Data and Fine-Tuning! Just Fold the Network to Compress — https://arxiv.org/abs/2502.10216
4. Byte Latent Transformer: Patches Scale Better Than Tokens — https://aclanthology.org/2025.acl-long.453/

**Implementations and technical documentation:**
- Sims2026 audited baseline — https://github.com/drdre72/Sims2026/tree/16fb91159fedd817aca09b930855fe212a6657ed
- Qwen3.5 model code, reference and current architecture — https://huggingface.co/docs/transformers/model_doc/qwen3_5
- Official Qwen3.5-4B — https://huggingface.co/Qwen/Qwen3.5-4B
- Folding research implementation — https://github.com/nanguoyu/model-folding-universal
- BLT research implementation — https://github.com/facebookresearch/blt
- PyTorch Windows/CUDA setup — https://pytorch.org/get-started/locally/
- Safetensors — https://huggingface.co/docs/safetensors/index
- MLflow local database documentation — https://mlflow.org/docs/latest/ml/tracking/tutorials/local-database/

**Independent review:** `Pasted text(2).txt`, uploaded with reviewer recommendations (not a peer-reviewed paper). v1.1 explicitly addresses edit-surface ambiguity, proposer specification, sample size/power, split leakage, incompatible substrates, enforced blinding, Windows performance, scheduling, semantic-memory trust, and citation hygiene.
