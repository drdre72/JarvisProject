# HORIZON: From Parameter Perturbation to Evidence-Guided Adaptation

**A critical synthesis of weight-space programming, mechanistic interpretability, memory, and resource-constrained intelligence**  
JarvisProject Research Program • Working paper HZN-LR-01 • 24 November 2026  
Status: literature synthesis and proposed roadmap amendments; not a new experimental result.

## Abstract

Horizon investigates three distinct hypotheses: structural model folding (F), gradient-free weight self-editing (W), and byte-level student construction (B). Recent public studies suggest that useful parameter changes can be authored through reasoning and that some internal computations admit causal descriptions. However, the ability to modify a model, preserve its operation, improve a task, and improve its editing competence are different claims. Horizon's own Qwen3.5-4B development results make this distinction concrete: 123 cumulative edits in a reconstruction did not collapse the model, but an apparent improvement on a repeatedly observed probe failed on held-out text; a subsequent controlled pilot found no convincing advantage over practice-adaptive search. This review identifies a missing common layer: evidence-guided localization and independent validation of changes, with separate representations for procedural knowledge, episodic memory and model parameters. It proposes a unified conceptual model and a targeted amendment to Roadmap v1.2 without changing experimental gates or claiming that the 4B threshold has been overcome. The result is a bounded, falsifiable theory of adaptation rather than a claim of general or unlimited intelligence.

**Keywords:** recursive self-improvement; weight-space programming; mechanistic interpretability; continual learning; model folding; byte-level distillation; causal localization; evaluation integrity.

## 1. Scope and method

This is a targeted narrative review, not a systematic review or meta-analysis. The cutoff is 24 November 2026. Sources were selected for direct relevance to Horizon's existing hypotheses, negative results, interpretability, memory, resource budgets and experimental controls. Primary arXiv HTML/abstracts, institutional research pages and repository documentation take precedence over media summaries. Recent preprints are treated as author-reported results, not independent replications. Better Stack (@betterstack) is a discovery source, particularly where its descriptions identify public implementations or demonstrations. The channel index and accessible description mirrors are incomplete; not every video or linked artifact could be inspected. No code was executed, no checkpoints modified, no sealed data opened and no repository changes made.

Internal baseline: JarvisProject `main`, `Horizon/ROADMAP_v1.2.md` and `preregistration.v1.2.yaml` (23 November), plus the Sims2026 `research/gate-1-acceptance` research reports [R1–R5]. The earlier attempt to locate the experimental branch in JarvisProject was a repository mismatch: the branch is in Sims2026. Raw local artifacts are gitignored and were not independently inspected. The paper therefore audits documented claims and design, not experimental reproducibility.

Evidence classes used throughout: **P** = public author-reported study; **I** = internal documented pilot; **D** = deduction from definitions or constraints; **H** = new hypothesis requiring confirmation; **L** = exploratory lead lacking adequate verification. A published paper can establish that a procedure was reported, not that the same outcome will occur in Horizon.

## 2. Horizon's present scientific position

Roadmap v1.2 separates task gain, editor capability, retention, efficiency and autonomy. It establishes a C0–C4 claim ladder, five evaluation partitions, transactional edits, and a hardware-stop rule. Those are valuable constraints; none should be weakened by an attractive new preprint.

| Internal evidence | What follows | What does not follow |
|---|---|---|
| Gate 0.5 A: 0/136 sparse probes collapsed; dense perturbations exhibited strong family/depth dependence. | The tested perturbations have different stability profiles. | A universal safe surface, or proof that all early-layer edits are dangerous. |
| Gate 0.5 B: 6 runs, 123 applied cumulative edits, no collapse; parseable edits were attenuation. | A structured protocol can elicit survivable edits from this 4B model. | A replication of the unrestricted published procedure, or a useful learning effect. |
| Seed-303: original PPL 7.460 → 6.983; held-out token-weighted PPL 4.780 → 4.834. | The observed probe gain failed to transfer to this text holdout. | All attenuation is harmful, or a definitive task-accuracy result. |
| Gate 1 v2: 24/24 parseable proposer outputs, 17/24 applied; sparse results near zero; all three self-proposer attenuation trajectories worsened selection CE. | Interface syntax improved; evidence of useful self-editing remains absent. | C1, superiority over search, or a general 4B impossibility theorem. |

Sources: [R2–R5]. The negative seed-303 result is particularly useful because it separates a real, reproducible parameter-induced effect from a useful effect. The original probe was also the feedback signal. The 24-passage validation set is now examined; it cannot serve as the future independent final set.

Gate 1 used short development passages, not a complete capability benchmark. The reported timings cover wall time around GPU calls, not device utilization. Comparators differ in perturbation matching and evaluation cost. The strongest practice-adaptive comparator is the right conceptual challenge, but the pilot cannot support a causal superiority conclusion. Existing confirmatory blockers remain.

**Diagnosis:** Horizon has demonstrated mechanisms for controlled perturbation and restoration. It has not demonstrated that its small model can identify an intervention direction associated with a useful transferable behavioral change. That is narrower and more informative than “4B cannot improve itself.”

## 3. The weight-space frontier: what is actually new?

### 3.1 External editors establish feasibility, not self-improvement

Takehi et al., *Sparks of Weight-Space Programming* (2 November) [1], report a frontier model directly interpreting and constructing network weights, including edits to Qwen3.5-0.8B that improve mathematical benchmark results from 7.9% to 48.1%. The motivating failure involves excessive reasoning loops. This is an existence demonstration that an editor can exploit structure in another network without training it conventionally. It does not establish that the edited small model could have discovered the same intervention. The project is particularly relevant because a small model's useful computation can be altered without increasing its parameter count.

Hu et al. (6 November) [2] broaden the study to 16 trajectories, seven benchmarks and two target sizes. Some changes transfer across tasks; six trajectories retain the original weights after candidate gains fail validation. This is important negative evidence. A useful research pipeline needs abstention as a normal outcome rather than obligatory improvement. Benchmark gains in selected successful runs are not an expected gain per attempted edit.

**Deduction (D):** Separate editability of a target from competence of an editor. Let E denote an editor, T a target, and U a fixed set of observation and intervention tools. Evidence for improvement under E_large → T_small says nothing sufficient about E_small → T_small. Neither target capacity nor editor capacity can be inferred from parameter count alone.

### 3.2 Fan et al.: a more precise failure taxonomy

*Language Models Can Rewrite Their Own Weights*, v2 (18 November) [3], reports failures at 2B, 4B and 9B and success at 27B within a specific Qwen3.5 protocol. The full HTML, especially sections 3.1 and E.2 and figures 11/23, provides more diagnostic detail than Horizon v1.2 acknowledges:

- The 4B model recognized the long-output symptom, but did not inspect hidden states. It pursued speculative changes, including query-head modifications, and continued despite failed paired tests. It ultimately withdrew its edits after changing many tensors.
- The 9B model used largely trial-and-error changes and submitted a damaging result despite adverse feedback; its tests were weak because they involved tasks it already failed.
- The 27B model used hidden-state contrasts associated with completion versus truncation, evaluated hypotheses against paired results, and adjusted its approach.

The source reports a +6.6-point held-out gain in a later protocol. Its repair experiment uses a healthy model to edit a damaged copy; that is not the same as a corrupted agent repairing itself autonomously. The public 4B result is also more accurately described as catastrophic **candidate** failure followed by withdrawal, rather than necessarily a retained failed deployment checkpoint. The 9B submitted edit did collapse on held-out tasks.

The distinction matters: Horizon Phase B did not reproduce the collapse in its structured five-operation interface, but that is not a contradiction of the published result. Different interventions, proposal format, decoding, observation data, task objective and stopping behavior are confounded. The 4B–27B finding supports a scale-dependent capability difference in the studied setup, not an architecture-independent hard threshold.

**Amendment:** replace the claim that the original paper gives almost no failure diagnostics with a qualified statement: it documents failure trajectories and reasoning/tool-use differences, while a fully controlled tensor-by-tensor and perturbation-matched failure analysis is still missing. Reframe Gate 0.5 as boundary mapping under an alternate protocol, not exact source replication. Preserve all historical files.

### 3.3 SELFWRITE: bounded recurrence, not increasing editor competence

Zhang and Li, *Recursive Self-Editing* (22 November) [4], provide the closest public counterpart to Horizon W. Their Qwen3.8-27B workflow uses fresh screening examples, white-box observations, rank-limited candidate updates and paired evaluation. Eight proposal sessions per generation compete for promotion. Five generations yield reported held-out avg@4 76.30% → 77.75%, pass@4 82.60% → 84.80%. The same model lineage supplies subsequent editors.

Important qualifications: the public design reuses a gate across generations, so gate gains are adaptively selected; the separate test set is the better evidence. Its result is math-specific and finite. A modified model that remains able to find useful changes is not necessarily a better editor than the unmodified model. The paper's inference-driven search also has significant proposal and evaluation costs, even without gradients.

Horizon already has the stronger conceptual safeguard: the G0/G1 editor × G0/G1 target crossover. Preserve it. A later model doing better on its own target may simply inherit an easier target or a helpful archive. Only a cost-matched editor comparison on common targets, with consistent observations, can identify a meta-capability effect. A useful external reference for intervention geometry does not authorize expanding the 4B study, opening holdouts or relaxing its hardware-stop condition.

### 3.4 Two additional high-priority publications from 19 November

Li et al., *WeightCode* [5], report programs that install new functions into trained networks, including command composition and robustness. Their cross-checkpoint results suggest a distinction between copying an edit and reusing the transformation that generates it. They also demonstrate sequential accumulation of specific capabilities in vision models. This is promising for a future catalog of documented transformation methods, but the external frozen editor, target tasks and architecture differ from Horizon. It is not evidence of recursive self-editing by a 4B LLM.

Skianis and Likas, *Weight-Space Programming Writes In-Context Learning and Matches Fine-Tuning in Language Models* [6], report that external frontier editors often derive task-related activation contrasts from few-shot versus zero-shot execution and encode them into weaker models. Some results rival conventional adaptation on the same examples, while wording variation and unfinished runs reveal limitations. This offers an important bridge between transient context and persistent procedural capability, not between all memories and weights. It also emphasizes that a near-unchanged perplexity score can coexist with a meaningful task behavior change.

**Cross-paper synthesis (H):** Structured observations may reduce the diagnostic burden on a bounded editor. The shared motif is not arbitrary numerical alteration; it is a meaningful behavioral contrast followed by localized, validated intervention. No public study cited here demonstrates that this suffices for Horizon's 4B self-editor. Treat it as a literature-derived hypothesis and retain a fixed non-model comparator, so the contribution of the model is not confused with the contribution of the diagnostic tool.

## 4. Mechanistic interpretability: the missing observation layer

### 4.1 Native neurons versus an arbitrary basis

Goodfire's *Neural Networks Are Already Clean* [7] is explicitly a working hypothesis. It argues that apparent entanglement can sometimes reflect the chosen level of analysis rather than intrinsically inscrutable computation. Tigges et al., *Neurons as an Interpretable Basis* (10 November) [8], investigate gated MLP units in Qwen3.6-35B-A3B and assess their activation patterns, subcircuits and functional descriptions. This is evidence in favor of native computational structure, not a theorem that every neuron is monosemantic or that every model admits a clean decomposition.

The relevant distinction is between a label and a causal role. A unit that activates with a concept may be a detector, a gate, an output contributor, a correlated byproduct or part of several mechanisms. A readable label alone cannot justify removing or modifying it.

### 4.2 Neuron communities and causal evidence

Shilov et al., *Discovering Neuron Communities in Language Models* (17 November) [9], use attribution across prompts, graph clustering and causal interventions to find groups in Llama-3.1-8B. Selected small groups account for aspects of arithmetic, copying, grammar and formatting in the layers examined. This bridges individual-neuron analysis and larger functional circuits. Their discovery process uses gradient-based attribution. Horizon's no-gradient rule therefore prevents silently importing the exact method into W. It can inform a separate interpretability study, or motivate forward-only observations as a different method; neither would constitute an exact reproduction.

This is a logical correction to an assumption common across F and W: **localization is not the same thing as sparsity.** A coherent direction may be implemented across many scalars, while an arbitrary sparse edit may touch unrelated computation. Likewise, neighboring or weight-similar neurons need not serve interchangeable roles.

### 4.3 Proposed observation vocabulary (conceptual, not an implementation authorization)

A research result should distinguish (a) the observed symptom; (b) the contextual conditions where it occurs; (c) a reproducible association between internal activity and the symptom; (d) a causal intervention demonstrating a role; (e) selectivity and side effects; (f) confidence and known limits. This vocabulary can be used for all three tracks without changing their experimental protocols. A sparse parameter count, norm bound, or layer label is a structural constraint; it is not semantic evidence.

**Gap resolution:** the current roadmap uses an edit whitelist as its main representation of safety. The public literature justifies adding *evidence quality* as a separate dimension of experimental interpretation. It does not justify changing the whitelist after examining results. Any such change belongs in a new version with separate comparisons and independent review.

## 5. Folding (F): functional redundancy versus numerical resemblance

Wang et al., *Forget the Data and Fine-Tuning! Just Fold the Network to Compress* (ICLR 2025) [10], use clustering to merge structurally similar units while addressing variance distortion. This is distinct from weight quantization, unstructured sparsity and recurrent-depth computation. The paper provides a credible foundation for F as structural compression. It does not establish that functional redundancy can always be identified by parameter similarity, or that smaller stored weights produce proportionally faster inference on an arbitrary runtime.

Goodfire's community results suggest a theoretical tension: units that contribute to the same function may be complementary, not substitutable. For example, one unit may provide a conditional gate while another contributes content. Merging them simply because they are co-active can destroy the very function one intended to preserve. A future F study should therefore distinguish structural similarity, functional equivalence, and complementary cooperation. These are different relations, not three names for redundancy.

NVIDIA's *Dory1.0-4B* (2 November) [11] introduces elastic depth using a shared recurrent block. Authors report that increased loops improve performance and sometimes shorten generated traces, with a last-loop KV-cache option. Dory is relevant to resource-constrained reasoning because it decouples stored parameter count from executed depth. It is not post-hoc folding, and looping requires extra layer computations. Its training scale also exceeds Horizon's local capacity. Use it as an architecture reference, not evidence that an existing Qwen checkpoint can be folded or looped without loss.

NCM [12], from Stanford Hazy Research, takes another route: hierarchical discrete latent representations generated in parallel within levels. The reported 21×, 10× and 31× results are relative to diffusion language-model comparators on three specific tasks; not a general speedup over autoregressive assistants. It is relevant to future B/F architecture design because granularity and parallelism can affect inference costs. It is a separate trained architecture, not a drop-in acceleration mechanism.

**Deduction:** memory footprint, arithmetic work and critical-path latency are independent resources. Quantization reduces representation cost; folding changes structural width; looping reuses stored weights while potentially increasing work; NCM changes the generation schedule. Compare them as separate mechanisms on quality/bytes/latency/energy frontiers, not a single compression percentage.

## 6. Byte building (B): representation, distillation and fair accounting

Marathe et al., *Breaking the Token Ceiling* (11 September) [13], study token, byte and byte+EOT students under controlled training regimes. They introduce approximate and exact conversions from token probabilities to byte probabilities. Token students are favored at low FLOPs; distilled byte students may overtake with sufficient training, with the strongest asymptotic performance claims extrapolated. The reported storage savings in teacher logits are not deployment latency gains. The study involves roughly 1B layer-parameter-matched models and very large training data volumes, so it cannot simply be scaled down to a consumer-GPU result.

The *Byte Latent Transformer* [14] provides an alternate architecture with adaptive grouping of bytes into patches. Its relevance is to the tension between fine-grained representation and expensive sequential processing. Byte granularity can improve language coverage, robustness or vocabulary economics, while longer sequences can increase costs. Hierarchical grouping is one way to address those costs but changes the architectural substrate. NCM's coarse-to-fine generation and BLT's input patching are related by abstraction, not identical algorithms.

Horizon already prescribes matched budgets, common bits-per-byte (BPB) and UTF-8 checks. Add explicit separation of teacher inference cost, student training cost, and deployment cost. Token-level cross entropy is not directly comparable across different tokenizations. BPB is useful where likelihoods over the same bytes are well-defined, but task correctness, latency per complete answer, memory and energy are still needed. EOT is a modeling event, not a literal byte in the user's text; accounting must avoid treating it as extra source data.

**Gap resolution:** B does not need W to succeed. A byte student may provide a future target or specialist; no evidence establishes that byte representation alone improves self-editing. Combining F/B/W requires a shared compatible substrate and its own controls. Do not treat their gains as additive by default.

## 7. Continual intelligence: select the correct place to learn

### 7.1 Context versus weights

Mirtaheri et al., *Where Should LLM Agents Adapt at Test Time?* (15 November) [15], evaluate 14 adaptation mechanisms with Qwen3.5-9B on 109 tasks from four environments and five task orders per environment. Their best in-context policy raises mean success from 28.8% to 49.4%; adding GRPO raises it to 51.6%. The weight adaptation in this study uses gradient-based methods, not W. The authors also observe that context reuse can discourage fresh verification, while adding reinforcement learning can restore some of that behavior.

This suggests two design lessons. First, adapting to an environment need not require persistent parameter updates. Second, an improvement in reuse efficiency can produce stale assumptions. A personal assistant must be able to retain information while checking that it is still true. An immutable permission rule, a changeable user's preference, and a learned general skill have different lifetimes and should not be stored or governed identically.

Xie et al., *From History to State* (v2, 6 November) [16], demonstrate constant-context skills using deterministic state tracking plus trained lightweight task modules. They report 2–7× reductions in prompt tokens per turn relative to controlled ReAct baselines. This supports keeping an explicit current-state representation rather than continually reinjecting a complete history. Its supervised and reinforcement training stages distinguish it from gradient-free self-editing. It is relevant to future Jarvis runtime design regardless of whether W reaches C1.

### 7.2 Memorilla: bounded context, unbounded external history is not infinite memory

Fallahpour et al., *Memorilla* (v3, 3 November) [17], encode source documents once and retrieve information into a query-conditioned latent memory interface for a frozen model. Sixteen memory embeddings achieve the strongest reported performance on five of eight benchmarks compared with the tested baselines. The FactKG result (84.33% versus 80.43% full-context inference) is interesting, but not evidence of perfect recall. TextWorld success rises from 18.6% to 24.2%, leaving substantial unsolved behavior.

A key conceptual distinction: a fixed-size memory representation for a query does not imply that the total external store, indexing time or encoding cost is constant. A latent summary is lossy and may not reliably preserve exact dates, identifiers or source wording. For Jarvis, the canonical record must remain retrievable, attributable, correctable and deletable. Latent compression can be an optional derived cache, not an authoritative record or a permission mechanism.

Better Stack's 15 November video tested the approach on an H100 and introduced additional retrieval tasks [18]. Its public description identifies the upstream paper and project; its demonstration is useful as independent exploratory pressure-testing, but not a controlled replication of all published experiments. Hardware and model compatibility must not be assumed for Horizon's A4500. The paper's own benchmark results and the video should remain separately attributed.

### 7.3 mini-AGI: an engineering lead, not an AGI result

The mini-AGI project [19] and Better Stack's September coverage [20] explore continual data ingestion with disk-resident experts on consumer hardware. This is a distinct mechanism from W because ongoing learning involves training. The project demonstrates a possible software architecture for a model whose total stored capacity exceeds active GPU memory. Its public README and demonstrations do not establish robust lifelong learning, generalized intelligence or absence of forgetting. Disk fetch latency, memory traffic, expert routing, growth limits, and evaluation on fresh environments remain open engineering questions.

The useful concept is tiered capacity: frequently used computation remains resident, other computation can reside in slower storage, and persistent factual records remain outside the neural model. This is consistent with Jarvis's portability goal, but it is not a justification for an unbounded expert growth policy.

### 7.4 A memory taxonomy for Jarvis

| State class | Example | Governing principle |
|---|---|---|
| Authoritative state | permissions, device identity, user-approved constraints | explicit, deterministic, outside mutable model weights |
| Episodic record | a dated observation or conversation | source-linked, editable/deletable, access-scoped |
| Semantic store | established facts and relationships | versioned, confidence- and time-aware, provenance preserved |
| Working state | current task, active tool result, current environment | bounded, refreshed, discarded when stale |
| Procedural skill | verified reusable workflow | versioned and scoped; independent validation before promotion |
| Neural parameters | distributed general computation | separately evaluated; immutable during ordinary deployment |

This taxonomy resolves an ambiguity in the concept of “persistent memory.” Persistence can occur in an external store, an executable skill or a model artifact; these differ in auditability, reversibility and retrieval precision. A complete Jarvis need not force every form of learning into one representation.

## 8. Evaluation science: preserving the ability to say no

### 8.1 GUARD and directional regret

Zhang et al., *GUARD* (17 November) [21], study harness adaptation rather than weight editing. Their framework tracks new failures separately from unresolved problems, pins established rules, and evaluates candidate harness changes before promotion. Its formal guarantee applies to the specified gate and self-review objective, not to arbitrary future tasks. It cannot replace Horizon's independent capability evaluator.

A direct conceptual transfer is to keep a paired transition ledger. For a given task and fixed scoring procedure, four outcomes are possible: both parent and candidate fail (unresolved); parent fails and candidate succeeds (acquisition); parent succeeds and candidate fails (regression); both succeed (retention). A simple net accuracy delta is acquisition minus regression divided by the number of tasks. It discards information about the two rates individually. Two candidates with identical net improvement can have very different risk profiles. This is a mathematical property, not an empirical hypothesis.

For example, a gain of three correct tasks with zero losses and a gain of 23 correct tasks with 20 losses both add three correct answers. A deployment policy cannot treat those as automatically equivalent. Protected behavior, severity and task-family scope matter. Retention is not a weighted average that can compensate for violating a non-negotiable constraint.

### 8.2 Control Map and update history

Zhao et al., *Beyond Update Counts* (26 October) [22], argue that evaluation must identify what updates preceded each scored answer. Immediate acquisition, later retention, unrelated-behavior preservation and transfer to new settings are separable. The sequence and placement of updates can matter even at equal counts. This complements the existing Horizon provenance DAG and patch ledger. The primary arXiv endpoint did not load during this review; metadata and accessible indexed descriptions were cross-checked. Treat details beyond the documented framework as unverified.

Horizon should describe edit attempts, valid proposals, applied changes, rejected changes and promoted checkpoints separately. A trajectory containing 24 proposal slots may involve different numbers of actual modifications. Candidate cost, parent identity and feedback exposure must travel with each observation. This is particularly relevant to Phase B's differing proposal budgets and batches, where applied-edit counts and proposal counts are not interchangeable.

### 8.3 Why a hidden gate is not enough

Selection can overfit even if the proposer never directly reads the selection corpus. Repeated accept/reject decisions and the survival of selected checkpoints disclose information indirectly. Likewise, comparing many candidates and retaining the best can produce an upward trajectory from noise. A gate score increasing over time is partly a consequence of the acceptance rule; it is not independent evidence of generalization.

The roadmap's rotating selection split, separate regression data, final one-time opening and explicit cost-matched controls already address much of this. Keep the final data independently administered. The source code and current development account do not provide a permission boundary merely because the proposer prompt omits the data. The final holdout is not a debugging tool.

Horizon has a strong opportunity to improve the *scientific contract* without touching model weights: define separately what counts as survival, task improvement, retention, transfer, and editor improvement. The minimum useful effect and tolerable regression margins must be specified before confirmation, supported by the pilot variance and practical use case. Do not retrofit the +3 percentage-point threshold or replace it with a small loss difference simply because the public SELFWRITE gain is smaller.

## 9. Unified framework: bounded adaptation across representations

Let the state of an experimental agent be described conceptually by four components: neural model W; explicit memory M; versioned skill/harness H; and external governance G. Let A be its behavior under a fixed observation and resource budget. Changes to W, M and H can alter A. Governance G defines the authority to make and approve changes; it is not part of the model's editable scientific search space.

The following common principles connect F, W and B:

**P1 — Representation specificity.** Store information in the most auditable representation appropriate to its purpose. A transient fact does not warrant a neural update simply because the model is capable of one. Structural compression, task-vector interventions and contextual memory are not interchangeable.

**P2 — Causal locality.** A small parameter count is not proof of a small behavioral effect. Prefer descriptions of conditional functional effects over numerical proximity alone. “Local” must be tested both within the network and on unrelated behavior.

**P3 — Separate competence dimensions.** Target capability, editing competence, tool-use competence, transfer and resource efficiency are different variables. A mechanism for reducing latency cannot by itself support a C2 claim. A system with better task accuracy may still be worse at selecting changes.

**P4 — Retention and abstention.** An experimental controller must be able to reject all candidate changes. Baseline performance and protected invariants are explicit constraints; failure to improve is a legitimate scientific result.

**P5 — Full resource accounting.** Include proposal generation, evaluation, memory retrieval and maintenance, conversion, and rollback. No-gradient does not mean no compute. Fewer stored weights or tokens does not necessarily imply lower latency or energy.

**P6 — Independence of evidence.** Observations used to propose or select a change cannot serve as the sole proof of its generalization. The target's own claim of improvement is not an independent measurement.

**P7 — Reversibility and external authority.** Model artifacts remain isolated in the research environment. Neither the experimental editor nor its descendants control the grader, permissions, production promotion or protected policy. Provenance is maintained externally; human approval is required for any change to runtime authority.

These principles do not imply a new neural algorithm. They define a common scientific language for interpreting otherwise heterogeneous results. They also explain why the roadmap's original tracks can continue to deliver value independently.

## 10. Gap register: what literature resolves, and what it does not

| Roadmap area | Gap or ambiguity | Proposed disposition |
|---|---|---|
| W / §0.1 | Scale result treated as a hard threshold | Correct to conditional finding in tested family/protocol. Resolved conceptually; threshold transfer untested. |
| W / §0.3 | Source failure diagnostics described as largely absent | Cite Fan v2 E.2 and figs. 11/23; retain missing matched perturbation analysis. Documentation fix now. |
| Gate 0.5 | Alternate structured protocol described as source reproduction | Distinguish reconstruction from direct replication. Documentation fix now. |
| Gate 1 | Safe surface conflated with useful surface | Document distinction among structural bounds, functional locality and task gain. No empirical guarantee. |
| Gate 1 | Feedback loss equated with general capability | Explicitly classify loss as a diagnostic. Seed-303 provides counterexample. Capability outcome still open. |
| Gate 2 | Repeated gains interpreted as improved editor | Preserve crossover, independent target comparison, cost-normalized outcome. No new experiment authorized. |
| F | Co-activity or similarity mistaken for redundancy | Separate similarity, equivalence and complementary computation. Actual compression effects untested. |
| B | Data efficiency mistaken for inference efficiency | Separate train/teacher/serve resources and common-unit reporting. Actual local performance untested. |
| F/B | Looped depth, compression, byte and parallel generation treated as one efficiency mechanism | Define them as distinct axes. No shared-substrate claim. |
| Stage II | Every learning event implies persistent neural adaptation | Adopt memory taxonomy and clarify trust levels. No runtime promotion implied. |
| Evaluation | Count of updates substitutes for transition history | Add conceptual ledger fields; distinguish paired regression/acquisition. Already substantially covered by v1.2. |
| Governance | “Self-improvement” implies authority to alter the system that judges it | Preserve external immutable controls and human approval. No authorization to change security boundary. |

The first three corrections are the most immediate because they improve the truthfulness of the current roadmap without requiring new GPU time. Other changes clarify the design, but do not establish their empirical effectiveness.

## 11. Proposed roadmap addendum (not a protocol change)

1. **Record literature and status:** attach this working paper under `Horizon/` after human review; keep existing versioned preregistration and results unchanged. Add citations for public primary sources, with version and publication dates. Mark internal pilot artifacts as documented but not independently reproduced by this review.
2. **Correct the scale prior:** retain a strong negative prior for 4B under the published procedure; remove language implying that a single family/protocol establishes a universal threshold. Document the difference between candidate collapse, submitted collapse and withdrawal.
3. **Add the conceptual observation layer:** distinguish symptom, association, causal evidence and intervention specificity in reports. Do not silently implement a new intervention surface or run a revised proposer as part of the existing gate.
4. **Clarify the metric contract:** report task acquisition, regression, retention, transfer and cost separately. Keep the independent evaluator and sealed final blockers. Loss-only pilots remain exploratory.
5. **Preserve independent tracks:** maintain F, W and B separately; categorize memory, Dory, NCM and mini-AGI as supporting architectural research, not additional active W arms. Continue only approved milestones under the existing resource and stopping constraints.
6. **Keep authorization boundaries:** literature, papers and code are untrusted research input; no downloaded script receives production credentials or access to sealed evidence. Do not use a model-generated proposal to revise the evaluator or override human approval.

No changes to parameters, thresholds, active candidate operators, hardware budgets or confirmation sample sizes are proposed here. Any future empirical revision needs a separately reviewed and versioned protocol; a literature argument cannot substitute for running it.

## 12. Better Stack source-tracing register

| Discovery item | Primary or upstream material | Evidence disposition |
|---|---|---|
| GPT-6 edits another model (3 Nov) | Takehi et al. [1] | External editing, not recursive competence. |
| AI changes its own weights (21 Nov) | Fan et al. [3]; related SELFWRITE [4] | Directly relevant; distinguish the two papers. |
| Memorilla (15 Nov) | [17], Stanford project and independent demos [18] | High-priority memory architecture; exact recall not demonstrated. |
| mini-AGI (30 Sep) | [19–20] | Engineering prototype; training-based; low evidence for AGI. |
| NCM explainer (21 Nov) | [12] | Primary comparator is diffusion generation, not general LLM speed. |
| Benchmark reliability (9 Jul) | [23] and independent evaluation literature | Discovery lead; video-specific claims not independently audited here. |
| Tiny chip trains AI in 18 seconds (22 Nov) | Channel index [24] | Unresolved primary-source linkage; exclude headline from scientific conclusions. |

Better Stack's value is the willingness to exercise unusual implementations and publish practical observations. A useful ongoing source index would retain the video URL, publication date, primary paper, code revision, hardware conditions, observed limitations and reproduction status. Secondary demonstrations must not become an extra independent scientific study merely by being mentioned in a video.

## 13. Conclusion

The public research converges on a more nuanced proposition than unlimited self-improvement: a model's behavior can sometimes be altered persistently through structured interventions, and some of the computations responsible for behavior can be localized. Larger editors have demonstrated forms of this ability that smaller models did not exhibit under comparable conditions. Horizon's documented experiments show that a constrained interface may change the survival behavior of a small model without producing a transferable capability gain. These observations are consistent rather than contradictory once the protocols and endpoints are separated.

The most valuable intellectual advance for Horizon is to treat evidence-guided localization, independently measured behavioral change, and representation-specific learning as common principles across its existing tracks. The most important unknown remains whether the 4B model can exploit such information to generate useful interventions beyond a matched non-model search. The literature does not resolve that question. Nor does it establish a reliable path from task improvement to improvement of the editor itself.

Horizon should therefore retain its claim ladder, its independent evaluation and its hardware-stop rule. A carefully documented null result is scientifically valuable; the broader Jarvis architecture can improve through trustworthy memory, efficient inference and independently validated procedural skills even if W remains inconclusive. The objective is not to make modification inevitable, but to establish when a change is worth retaining.

## References — public sources

All links accessed or checked against accessible indexed metadata on 24 November 2026. Preprint version numbers are included where established. The references support the reported findings; deductions and proposals are those of this working paper.

[1] Takehi, R., Teoh, J., Zhang, A., & Khattab, O. (2026-11-02). *Sparks of Weight-Space Programming: LLMs can natively read & write neural network weights*. MIT CSAIL research blog. https://rikiyat.github.io/blog/weight-space-programming/

[2] Hu, J., et al. (2026-11-06). *Exploring Weight-Space Programming: Capabilities, Generalization, and Limitations*. arXiv:2611.09382v1. https://arxiv.org/abs/2611.09382v1

[3] Fan, J., et al. (2026-11-18). *Language Models Can Rewrite Their Own Weights*. arXiv:2611.21885v2. Full text: https://arxiv.org/html/2611.21885v2

[4] Zhang, L., & Li, Y. (2026-11-22). *Recursive Self-Editing: LLMs Program Their Own Weight Edits for Self-Improvement*. arXiv:2611.29513v1. https://arxiv.org/html/2611.29513v1

[5] Li, B., et al. (2026-11-19). *WeightCode: Installing New Capabilities into Neural Networks via Weight-Space Programming*. arXiv:2611.25476. https://arxiv.org/abs/2611.25476 ; code linked by authors: https://github.com/Gen-Verse/WeightCode

[6] Skianis, K., & Likas, A. (2026-11-19). *Weight-Space Programming Writes In-Context Learning and Matches Fine-Tuning in Language Models*. arXiv:2611.26841. https://arxiv.org/abs/2611.26841

[7] Goodfire (2026). *Neural Networks Are Already Clean*. Informal research essay. https://already-clean.goodfire.com/

[8] Tigges, C., Kowal, M., McGrath, T., Lewis, O., & Cammarata, N. (2026-11-10). *Neurons as an Interpretable Basis*. Goodfire. https://www.goodfire.com/research/neurons-as-an-interpretable-basis

[9] Shilov, I., Fel, T., Haklay, T., Bushnaq, L., & Lewis, O. (2026-11-17). *Discovering Neuron Communities in Language Models*. Goodfire. https://www.goodfire.com/research/neuron-communities

[10] Wang, D., Šikić, H., Thiele, L., & Saukh, O. (2025). *Forget the Data and Fine-Tuning! Just Fold the Network to Compress*. ICLR 2025, arXiv:2502.10216v2. https://arxiv.org/abs/2502.10216v2

[11] Fishman, M., et al. (2026-11-02). *Dory1.0-4B: A Looped Normalized-GPT*. arXiv:2611.03998. https://arxiv.org/abs/2611.03998 ; model card: https://huggingface.co/nvidia/NVIDIA-Nemotron-Labs-Dory1.0-4B-BF16

[12] Kumbong, H., et al. (2026-10-09). *Next-Concept Modeling: Fast, Parallel Coarse-to-Fine Text Generation*. arXiv:2610.14162v1. https://arxiv.org/abs/2610.14162v1 ; https://hazyresearch.stanford.edu/blog/2026-10-12-ncm

[13] Marathe, K., et al. (2026-09-11). *Breaking the Token Ceiling: Distilling Smaller, Stronger Byte Models*. arXiv:2609.12303v1. https://arxiv.org/abs/2609.12303v1

[14] Pagnoni, A., et al. (2025). *Byte Latent Transformer: Patches Scale Better Than Tokens*. ACL 2025. https://aclanthology.org/2025.acl-long.453/ ; implementation: https://github.com/facebookresearch/blt

[15] Mirtaheri, P., et al. (2026-11-15). *Where Should LLM Agents Adapt at Test Time? In Context, in Weights, or Both*. arXiv:2611.19700v1. https://arxiv.org/abs/2611.19700v1

[16] Xie, H., et al. (2026-11-06). *From History to State: Constant-Context Skill Learning for LLM Agents*. arXiv:2605.05413v2; accepted to NeurIPS 2026 per authors. https://arxiv.org/abs/2605.05413v2

[17] Fallahpour, A., et al. (2026-11-03). *Memorilla: Latent Semantic Memory for LLMs*. arXiv:2610.21375v3. https://arxiv.org/abs/2610.21375v3 ; https://memory.stanford.edu/

[18] Better Stack (2026-11-15). *Did Stanford Just Solve AI Memory? (Memorilla)*. Video description mirror: https://gridvids.com/better-stack/did-stanford-just-solve-ai-memory-memorilla ; upstream code: https://github.com/snap-stanford/memorilla ; linked demonstration lead: https://github.com/andrisgauracs/memorilla-demos

[19] volotat. *mini-AGI*. Public experimental project. https://github.com/volotat/mini-AGI

[20] Better Stack (2026-09-30). *This Tiny AI Never Stops Learning (mini-AGI)*. https://www.youtube.com/watch?v=L-6O7R71CCg

[21] Zhang, M., et al. (2026-11-17). *GUARD: Stabilizing Recursive Self-Improvement with Regret Guidance*. arXiv:2611.23476v1. https://arxiv.org/html/2611.23476v1

[22] Zhao, Z., et al. (2026-10-26). *Beyond Update Counts: A Control Map for Evaluating Test-Time Model Updates*. arXiv:2610.36109v1. https://arxiv.org/abs/2610.36109 ; accessible metadata mirror: https://borowiecki.dev/abs/2610.36109

[23] Better Stack (2026-07-09). *AI Benchmarks Are Fake!?* Discovery lead. https://www.youtube.com/watch?v=pHAbwL7w83Q

[24] Better Stack channel and indexed upload listing (accessed 2026-11-24). https://www.youtube.com/@betterstack ; https://gridvids.com/better-stack

## References — internal project documents

[R1] JarvisProject, `Horizon/ROADMAP_v1.2.md` and `Horizon/preregistration.v1.2.yaml`. https://github.com/drdre72/JarvisProject/tree/main/Horizon

[R2] Sims2026, `gate0_5_taxonomy.md`. https://github.com/drdre72/Sims2026/blob/research/gate-1-acceptance/ai-research/research/gate0_5_taxonomy.md

[R3] Sims2026, `gate0_5_phaseb_selfedit.md`. https://github.com/drdre72/Sims2026/blob/research/gate-1-acceptance/ai-research/research/gate0_5_phaseb_selfedit.md

[R4] Sims2026, `seed303_validation_report.md`. https://github.com/drdre72/Sims2026/blob/research/gate-1-acceptance/ai-research/research/seed303_validation_report.md

[R5] Sims2026, `gate1_v2_open_protocol.md`. https://github.com/drdre72/Sims2026/blob/research/gate-1-acceptance/ai-research/research/gate1_v2_open_protocol.md

*End of working paper. No original experimental claims or changes to preregistration are authorized by this document.*
