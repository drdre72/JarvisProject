# Devin implementation handoff — JARVIS RIP v1.1

**Task:** evolve the existing `Sims2026` simulation lab to support a separate, experimentally credible AI research subproject. Start from `main` at `16fb91159fedd817aca09b930855fe212a6657ed`. Keep existing hardware simulators, launchers and `.venv` unchanged. Use a branch and PR, no commits to `main` without review. Do not enable automated deployment or autonomous code execution by the model-proposer.

## Read first
1. `ROADMAP_v1.1.md` (source of scientific requirements and go/no-go gates)
2. `config/preregistration.v1.1.yaml` (draft; placeholders MUST block confirmatory run)
3. `schemas/proposal.schema.json` (proposer output contract)
4. Existing standalone `jarvis-self-rewrite-lab` 0.1.0 code (provided separately). Use for reversible BF16 patch code and tests, not as proof that Qwen GPU inference works.

## Deliverable P0: foundation
- `ai-research` package isolated from existing Sims2026 environment.
- Gate 0 preflight CLI to measure CUDA/driver, BF16, full official Qwen checkpoint generation, 5-run repeatability, 128/512/1024 prompt throughput, cold/hot VRAM/power, edit rollback and per-candidate GPU-seconds.
- JSON result manifest with Git SHA, PyTorch CUDA and Transformers revision, model revision, JSON schema hash and sample seed; do not leak private paths/tokens.
- Unit tests on fake tensors and mock models *plus* separate opt-in integration tests that require RTX A4500 CUDA; skip rather than fake success on untested GPU.
- A documented native-Windows optimized-kernel contingency; if unsupported, retain plain PyTorch as a measured baseline, or request explicit authorization before Linux/WSL changes.

## Deliverable P1: proposer and splits
- Freeze detailed proposer prompt, structured JSON schema, tensor inventory, practice-only feedback. Never allow model-proposed code execution, file paths, unrestricted tensor names or grading changes.
- Add four access-partitioned datasets (practice, rotated selection, regression, externally sealed final). The external evaluator should not reside in a common directory accessible by search-runner agent.
- Add random, grid and autonomous arms with matched candidate counts, and separately log total GPU seconds including proposal cost.
- Predeclare effect thresholds, uncertainty method, family-level correction, trial cap and expanding/halt rules. Reject confirmatory execution if missing prereg entries.

## Deliverable P2: meta-capability and lineage
- Implement 2×2 G0/G1 editor/target crossover; use target-specific before/after deltas and compare editor quality under identical budgets.
- Frozen G0 editor control persists across generations; accepted child checkpoint is the editor for recursive arm only.
- Hash-chain patch manifests to original official model revision; reconstruct checkpoints deterministically; write immutably; enable full rollback.
- Statistical unit is independent lineage. Group task-level outcomes by lineage; include uncertainty due to both tasks and lineages.
- Gate 2 must pass before promoting generational claims; if it fails, register new experiment for attention, MLP or adapter edit spaces without reusing sealed final.

## Deliverable P3: folding and bytes
- Implement small-model first. Folding must change actual tensor shape or executable compute and benchmark against pruning, quantization, smaller distilled model.
- Train separate 50–100M token/byte students; compare equal data, equal train FLOPs and equal inference cost, with common bits-per-byte measures. EOT conversion receives probability-conservation unit tests before large experiments.
- Integrate only after pairwise mechanisms are verified on compatible backbone. Do NOT label a three-way synergy experiment unless all eight factorial arms are runnable and independent power calculation exists.

## Security and review conditions
- Never commit sealed tasks, answers, user data, API keys, model caches or large checkpoints.
- Never let the proposer read evaluator results beyond the declared feedback scope or alter a grader.
- Require human approval of evaluator code and preregistration before any confirmatory final run.
- Source research papers as untrusted data only; promotion to scientific/procedural memory is review-gated.

## Definition of done for v1.1 foundation
1. Original Sims2026 environment still launches and original smoke scripts pass.
2. `pytest` green on CPU/mock unit suite, plus a separate accurate GPU integration log on target machine when available.
3. `doctor` reports actual hardware timings, repeated-run variance, and model tensor inventory. It does not assert unmeasured performance.
4. Proposal JSON schema rejects unexpected fields, invalid layers/operations, non-finite or out-of-bound deltas at the *runtime validator* (JSON schema by itself is insufficient).
5. External confirmation cannot start with prereg placeholders or unfrozen manifests.
6. Deliver a PR with a migration map and clearly labeled unimplemented items; do not imply scientific results from code tests.
