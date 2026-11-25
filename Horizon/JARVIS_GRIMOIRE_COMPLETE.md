# THE GRIMOIRE — VOLUME I
## Unconventional computation, verified methods and experimental knowledge for JARVIS

**Edition:** 1.0  |  **Authored:** 2026-11-23  |  **Authority:** Research only

> *The marginal note matters only after the experiment proves it works.*

This volume translates difficult, sometimes neglected ideas from programming languages, compilers, symbolic reasoning and AI research into tests that a resource-conscious offline assistant could perform. It is not a list of magical optimizations and it is not a collection of executable prompts. Each entry distinguishes source-supported facts, our extrapolation, a falsifiable test, and conditions for safe deployment.

## Our central thesis

The most promising near-term route to a JARVIS that appears to learn continuously is **not** recurrently editing its neural weights. It is acquiring small, validated executable abstractions that reduce repeated reasoning and work across tasks. An external skill library is inspectable, reversible and testable. Neural weight editing may eventually complement that library, but is a separate scientific hypothesis governed by Horizon.

## Volume's three pillars

| Pillar | Source | Marginal annotation for JARVIS | Initial gate |
|---|---|---|---|
| **Change the language** | Let Over Lambda + On Lisp | Replace repeated prompt/tool orchestration with a typed task DSL and reusable combinators; no arbitrary dynamic eval | EXP-001 |
| **Change the computation** | Hacker's Delight + egglog + STOKE | Explore bit-level representation and equivalent algorithms with explicit machine-semantic tests | EXP-002, EXP-003 |
| **Change what is learned** | DreamCoder + LILO | Learn reusable functions from solved problems, document them, then reuse them on held-out tasks | EXP-004 |

These are complementary, not interchangeable. Macros alter how programs are expressed; e-graphs search equivalence classes; program induction discovers abstractions. None is, by itself, proof of self-improving general intelligence.

## Research shelf — annotated entries

| Entry | Lead | What it changes | Priority | Proposed domain |
|---|---|---|---|---|
| [GRM-001](entries/GRM-001.md) | Let Over Lambda | Language construction via macros | A | Task DSL / orchestration |
| [GRM-002](entries/GRM-002.md) | On Lisp | Bottom-up abstraction | A | Reusable reasoning routines |
| [GRM-003](entries/GRM-003.md) | Hacker's Delight | Efficient bit-level algorithms | A | Cache / indexing / codecs |
| [GRM-004](entries/GRM-004.md) | Metaobject Protocol | Controlled runtime extensibility | C | Future plugin architecture |
| [GRM-005](entries/GRM-005.md) | miniKanren / Prolog | Relational inference | B | Constraint-based planning |
| [GRM-006](entries/GRM-006.md) | egg / egglog | Equality saturation | A | Restricted expression optimization |
| [GRM-007](entries/GRM-007.md) | DreamCoder | Wake/sleep library learning | A | Acquired skill library |
| [GRM-008](entries/GRM-008.md) | LILO | Synthesis + compression + documentation | A | Readable, reusable skills |
| [GRM-009](entries/GRM-009.md) | STOKE | Stochastic superoptimization | C | Isolated native kernels |
| [GRM-010](entries/GRM-010.md) | AlphaEvolve | Evaluation-guided evolutionary search | B | Bounded algorithm discovery |
| [GRM-011](entries/GRM-011.md) | Lean 4 / Rosette | Proof and solver-aided search | B | Checked transformations |
| [GRM-012](entries/GRM-012.md) | Datalog / egglog | Incremental symbolic rules | B | Consistency and provenance |
| [GRM-013](entries/GRM-013.md) | Stitch | Efficient corpus-guided library compression | A | First practical skill-learning pilot |

**New practical insight:** Start the library-learning pilot with a small custom reference compressor and **Stitch**, rather than installing DreamCoder or LILO wholesale. Stitch has a Rust implementation and Python bindings; its reported advantages are workload-dependent and require independent measurement on JARVIS tasks.

**A = prioritize after isolation/validation. B = promising adjunct. C = defer until a concrete bottleneck warrants the complexity.** These are engineering priorities, not measured rankings.

## What LLMs underuse

“Underused” means these methods often are **not invoked by default in conventional chat-based or coding-agent workflows**; not that models are unaware of them or that no one uses them. The underutilization arises from poor tool interfaces, limited training examples, absent reliable evaluation, solver/environment overhead, and the fact that free-form neural outputs do not automatically constitute verified programs.

For JARVIS the missed opportunity is a heterogeneous system: local LLM for semantic interpretation; deterministic DSL for repetitive plans; relational logic for constraints; equality saturation for safe restricted rewrite search; independent test/proof harness for acceptance; durable library and provenance for reuse.

## The first campaign, in dependency order

1. **Gate G0 — Library integrity.** Validate metadata, stable IDs, sources, bounded retrieval and trust labels. No experiments, no GPU, no JARVIS modifications.
2. **Gate G1 — Compact task language.** EXP-001: compare ordinary Python imperative task plans to typed compositional plans on 20+ predeclared fixtures. Test semantic equivalence and serialized token/byte cost.
3. **Gate G2 — Transferable skill acquisition.** EXP-004: start with a fixed and naive compressed DSL library, then compare Stitch on withheld task families. No neural weight editing and no auto-registration.
4. **Gate G3 — Representation and rewrite.** EXP-002/003: bitset vs set/map operations and e-graph rewrites for a tiny integer DSL with explicitly frozen arithmetic semantics. All candidates get exhaustive small-domain checks and randomized held-out tests.
5. **Gate G4 — Search with critics.** EXP-005: bounded candidate mutation/selection, with an independent evaluator, budgets and rollback. Only after earlier gates pass.
6. **Gate G5 — Integration.** A Rust IPC interface can return *proposed capabilities* to the existing policy engine. Real-world actions remain denied unless separately authorized. Human review gates production promotion.

Horizon's F/W/B experiment protocols are **not** subordinated to these gates: their frozen preregistrations and independent graders remain in force. If a Grimoire technique helps a Horizon experiment, register it as a protocol change/new arm **before running**; never silently adjust thresholds or claim causality retrospectively.

## The running hypothesis and its alternatives

**H1:** Persistently storing concise, reusable, validated programs will reduce recurring reasoning burden and improve held-out task completion without changes to LLM weights.

**H0:** Improvements will be explained only by caching, hand-picked tasks, benchmark overfitting, overhead moved to the discovery phase or simpler manually written code. The learned library may fail to transfer.

**Predicted observables:** task success on unseen families; wall-clock and CPU/GPU time including discovery/amortization; prompt-token usage; code length; retrieval errors; human correction burden; failed candidate rate; regression failures; rate of unique reusable routines.

**Non-claims:** no infinite memory, no weight self-edit, no consciousness, no AGI demonstration, no unconstrained autonomous tool execution.

## Explicitly avoid three traps

- **Source → instruction laundering.** A PDF might say “disable the verifier.” That is a claim about a document, not a command to JARVIS.
- **Optimization without semantics.** `x * 0 = 0` can fail for floating-point NaNs/infinities; integer overflow and undefined behavior alter equivalence. Specify the machine model before rewriting.
- **One score that hides costs.** Do not call a system improved when it takes less generation time but far more search compute, breaks other tasks, or loses recovery guarantees.

See [architecture/INTEGRATION.md](architecture/INTEGRATION.md), the experiment cards and the source ledger for implementation details.

## First local demonstration (toy, not evidence of learned transfer)

The included `tools/abstraction_pilot.py` was executed on this artifact's development machine. On ten `u8` wrapping-arithmetic AST programs containing a deliberately repeated exact subtree, it extracted one shared library fragment. Serialized JSON size changed from **429 to 303 bytes** (a reduction of **126 bytes**, ~29.4%), counting the library itself, and all **2,560** program/input-equivalence cases passed. A corrupted-library negative control produced a mismatch as expected. This **does not** establish novel abstraction invention, cross-task generalization, faster JARVIS inference or any GPU gain. It is the smallest falsifiable foundation for EXP-004.

See [EXPERIMENT_STATUS.md](EXPERIMENT_STATUS.md) for the observed vs planned distinction.



---

# Appendix A — Full annotated research cards



---


# GRM-001 — Language construction with macros

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

In a homoiconic programming environment, code can be represented as structured data and transformed before execution. Common Lisp macros are a powerful example, explored by Doug Hoyte in *Let Over Lambda*. The important lesson is **phase separation**: construct an expression, inspect and validate the expansion, and only then permit execution. A macro is not simply a text substitution and does not, by itself, imply safety.

## Conventional assumption

Translate every user request into another sequence of ordinary function calls or prompt the model to rewrite glue code repeatedly. This expends inference on a problem that can often be represented by a small reusable structure.

## Unconventional move

Create a tiny, deliberately limited JARVIS task language whose expressions are **data**, not arbitrary Python. Its interpreters can be specialized for rendering, scheduling, validation and dry-run simulation. The LLM proposes an abstract syntax tree (AST); Rust validates typed operators and permissions. Design with an allowlist, not with eval/exec.

Example abstract plan (illustrative data, not runnable commands):

```json
{"plan":"sequence","steps":[{"op":"lookup_approved_note","topic":"battery"},{"op":"summarize","max_chars":500},{"op":"present_draft"}]}
```

Its advantage is not a guarantee of speed; it is that the language gives the model a smaller, testable output space and moves repetitive logic out of generation.

## Translation to JARVIS

Use the existing intent gate and Rust `policy.rs`. Introduce only read-only research/task AST nodes in shadow mode. Define operator contracts for input/output types, provenance and maximum work. Keep the compiled task description distinct from approval to perform actions. Unrecognized operators are invalid; model-produced text is never executed as a host-language program.

## Experiment

**EXP-001.** Build 20+ fixed synthetic tasks. Compare (a) explicit Python orchestration, (b) JSON task DSL compiled to the same trusted functions, and (c) repeated LLM-generated glue. Measure correctness, serialized representation size, model calls, parser rejection, developer maintenance cost and total latency including AST validation. Test unknown operators, nested-depth limits and malicious strings.

## Failure modes

Macro hygiene errors, accidental capture, nontransparent expansions, AST overcomplexity, a false feeling of sandbox security, and authorization bypass when an AST maps to side effects. A smaller representation is not proof of general intelligence. The real Common Lisp macro system should **inform the DSL design**, not be embedded into the voice runtime.

## Evidence and limitations

The book supports expressive code transformation. Whether a specific JARVIS task DSL reduces latency is **unmeasured**. Any improvement will depend on workload and implementation.

## Sources

SRC-001, SRC-002.

## Promotion condition

Typed semantics and parser fuzz tests pass; no unauthorized action can be requested; benchmarked held-out tasks show utility without regression. Human review required before adding an operator.



---


# GRM-002 — Bottom-up programming and reusable abstractions

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

*On Lisp* advocates building powerful higher-level abstractions from small composable functions. Instead of approaching each problem from scratch, identify invariants and produce reusable combinators. The Grimoire's marginal claim is that JARVIS should **reduce the number of distinct plans it needs to invent**.

## Conventional assumption

When a task changes slightly, regenerate the entire plan and interpret the result from scratch. This makes robustness depend on prompt wording.

## Unconventional move

Construct a catalog of typed **combinators** such as `filter_verified_sources`, `bounded_extract`, `summarize_cited_results`, and `compare_claims`. Permit higher-order compositions only where the type checker can bound side effects, cardinality, recursion and costs.

## Translation to JARVIS

A small library of deterministic read-only routines could support recurring tasks such as answering capability questions, retrieving user-approved notes, and producing documented research summaries. Every combinator carries version, contract, cost expectation, provenance and test corpus. JARVIS may choose an approved compositional plan, but cannot manufacture authority to register one.

## Experiment

Part of **EXP-001**. Compare reusable combinator plans against bespoke generated plans using predeclared task families and unseen paraphrases. Count repeated code, task success, instruction/plan length, runtime, and how often a human must correct the plan.

## Failure modes

Abstraction overhead, too-general operators, hidden global state, leaky permissions and premature generalization. A short plan can invoke expensive operators; measure actual resource use, not only token counts.

## Evidence and limitations

Bottom-up design is a mature programming methodology; transfer benefits in this assistant remain a hypothesis until measured. Retaining a function in a library differs from a model internally learning it.

## Sources

SRC-002.

## Promotion condition

Library candidates must have explicit contracts, isolated unit tests, no hidden I/O and demonstrated reuse across independently held-out task families.



---


# GRM-003 — Bit-level algebra and representation changes

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

*Hacker's Delight* documents bit manipulation, arithmetic identities and exact low-level transforms. Its distinctive point is that **the representation of a problem can dominate its cost**. For integer sets over bounded universes, bitsets sometimes replace many allocations, hash lookups and branches.

## Conventional assumption

Every membership or feature relation must use a general-purpose hash set, list or dictionary. Generic containers are often appropriate, but they are not always the cheapest choice.

## Unconventional move

Try a bounded bitset representation for repetitive JARVIS capability masks, rule predicates or small graph adjacency rows. A hypothetical 64-capability system can represent an allowlist in one `u64`; intersection becomes bitwise AND. This is an optimization of **representation**, never of authorization logic.

Example for a tiny fixed universe:

```rust
let allowed: u64 = 0b_0101;
let requested: u64 = 0b_0001;
let permitted = (requested & allowed) == requested;
```

The example is not permission infrastructure: it illustrates an exact relation assuming indices have already been safely mapped.

## Translation to JARVIS

Test bitsets for local search indexes or **internally derived** capability features. Keep the existing Rust policy engine as the sole authority; never let a generated mask bypass capability registration or consent checks. Compare `u64`/`Vec<u64>` to ordinary sets where cardinality and update frequency are known.

## Experiment

**EXP-002.** Freeze datasets of sparse and dense feature relationships; compare sets, sorted arrays, bitsets and compressed bitmaps on membership, union, intersection, updates and serialization. Check duplicate handling, missing IDs, zero-length universes, upper-bound indices, portability, memory, cold/warm timing and worst-case behavior.

## Failure modes

Signedness, integer width, shift-by-width, wrong index mapping, endian assumptions, aliasing, sparsity causing wasted space, overflow, and assuming microbenchmark wins matter to end-to-end latency. Hardware instruction tricks may not beat optimizing compiler output.

## Evidence and limitations

The published techniques are established; their benefit on JARVIS's actual access patterns is not known. Do not claim CPU savings until comparing compiler-optimized baselines with identical semantics.

## Sources

SRC-003, SRC-014.

## Promotion condition

Exact equivalence on full small domains and randomized large data; stable measurable end-to-end improvement with no policy bypass and no memory regression.



---


# GRM-004 — Metaobject protocols and controlled reflection

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

A metaobject protocol exposes selected mechanisms of an object system—such as method dispatch or class behavior—as programmable, introspectable interfaces. *The Art of the Metaobject Protocol* shows that a system can be designed to let extensions modify aspects of its semantics explicitly rather than by monkey-patching arbitrary internals.

## Conventional assumption

Every new JARVIS capability requires an invasive rewrite of dispatcher logic and manually synchronized UI behavior.

## Unconventional move

Make capability *descriptions* first-class typed objects: input schema, permissions, provenance, deadlines, deterministic fallback, source of authorization, health check and revoked state. An extension can register a declaration for review without being able to activate it.

## Translation to JARVIS

The current `scripts/capability_registry.py` and Rust `policy.rs` already give a foundation. A future metadata layer could describe capabilities consistently to the UI and planner. Preserve the policy engine's monopoly on authorization. Avoid runtime monkey-patching or rewriting dispatch methods.

## Experiment

In a branch, define two simulated read-only capabilities with the same declarative contract. Measure schema validation, rejection of missing permissions, revocation checks, type compatibility and recovery after a plugin fault. Use fixtures only; no OS access.

## Failure modes

Reflection can make changes hard to audit. A flexible protocol may enable extensions to interfere with trust boundaries, bypass event journals, or change dispatch at runtime. This entry is **deferred** until there is a real extension bottleneck.

## Evidence and limitations

The source establishes MOP as a design method, not that dynamic self-modification would be safe in JARVIS.

## Sources

SRC-004.

## Promotion condition

Formal capability contracts, deterministic loading, human-reviewed registration, explicit revocation and no observable unauthorized side effects under adversarial tests.



---


# GRM-005 — Relational programming instead of procedural guessing

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

Relational languages express constraints among objects and ask for assignments that satisfy them. miniKanren and Prolog illustrate a different computational direction: some predicates can be queried in more than one direction, making it possible to solve for missing inputs rather than manually tracing only forward steps.

## Conventional assumption

Ask the LLM to narrate a plan and trust that it maintained every scheduling, capacity and dependency constraint in text.

## Unconventional move

Turn the finite portion of the plan into explicit constraints and let a small solver enumerate valid assignments. Ask the LLM to explain *the solver's result*, with a separate indication of which constraints were actually enforced.

## Translation to JARVIS

Possible early tasks: nonconflicting research-worker slots, finite tool budgets, prerequisite ordering, and choosing experiments that fit available compute. For scheduling finite-domain tasks, SWI-Prolog CLP(FD) or a simple Python backtracking baseline may suffice. The original NLP remains responsible for eliciting constraints; the solver is responsible for constraint satisfaction.

## Experiment

Use predeclared finite scheduling fixtures, some satisfiable and some impossible, with an exhaustive reference enumerator for tiny cases. Record solution validity, missed possibilities, runtime, false claims, and how the solver explains `unsatisfiable` without fabricating a plan.

## Failure modes

Nontermination, search-space explosion, incompletely modeled real-world constraints, incorrect constraint translation, and unsound human claims about completeness. Relational code is not automatically efficient or correct.

## Evidence and limitations

The techniques are mature, but natural-language-to-constraint translation can still fail. No local Prolog installation is assumed for this package.

## Sources

SRC-005, SRC-013.

## Promotion condition

Independent fixture validation, bounded search, transparent unsat diagnostics, error handling and no externally consequential scheduling without normal JARVIS approval.



---


# GRM-006 — Equality saturation with e-graphs

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

Ordinary compilers often apply one rewrite at a time and commit to a path. Equality saturation uses an e-graph to maintain many equivalent expressions simultaneously, then extracts a candidate according to a cost model. `egg` and `egglog` are relevant Rust implementations; egglog adds logic-programming-style relations.

## Conventional assumption

Prompt the LLM to produce “the fastest equivalent program,” then run the first plausible suggestion. This often relies on an unverified equivalence claim and a poor mental cost estimate.

## Unconventional move

Declare a *tiny typed expression language* and a set of sound rewrite rules, saturate under explicit time/node limits, and extract candidates with separate costs (AST length, predicted instruction count, measured wall time). The LLM may **suggest a rewrite**, but the library only accepts it after semantic validation.

## Translation to JARVIS

The ideal first target is synthetic pure integer expressions, **not neural-network weights and not arbitrary Rust/Python source**. Start with `u8`/`u16` wrapping arithmetic and bitwise operations whose semantics are frozen, then compare with a simple deterministic baseline optimizer. A host-language AST frontend can be added only after type/overflow rules and side effects are explicitly modeled.

Example of a **sound** identity under unsigned wrapping arithmetic: `x + 0 = x`. A dangerous overgeneralization is treating identities for real numbers as valid for all floating point with NaN, infinities or signed zero.

## Experiment

**EXP-003.** Predeclare rewrite rules and cost function; compare baseline tree simplification vs equality saturation on a small random AST corpus. Exhaustively evaluate all 256 `u8` inputs for one-variable expressions; use randomized multi-variable holdouts and a second independent evaluator. Log saturation size, runtime, extracted cost and correctness. Require fixed seeds and time/node caps.

## Failure modes

Unsound rewrite rules, nonterminating/e-graph explosion, broken extraction cost assumptions, overflow mismatches, side effects, floating-point identity traps and overfitting the extractor to toy benchmarks.

## Evidence and limitations

E-graphs and equality saturation are established; whether they improve JARVIS workload latency depends on overhead amortization. `egglog` is promising but adds an optional experimental dependency, not a production prerequisite.

## Sources

SRC-006, SRC-015.

## Promotion condition

All candidates are semantically equivalent for the declared language, resource limits hold, and there is a measured benefit on unseen workloads including solver overhead.



---


# GRM-007 — DreamCoder wake–sleep library learning

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

DreamCoder alternates program-search activity with learning phases that consolidate commonly useful program fragments into a reusable library, while recognition mechanisms help guide later searches. The result can be a more expressive search vocabulary rather than merely a larger collection of answers.

## Conventional assumption

When JARVIS solves a problem once, store the conversation and hope retrieving it will help next time. A narrative memory preserves context but does not necessarily preserve an executable reusable operation.

## Unconventional move

From solved **synthetic** tasks, extract candidate functions with explicit signatures and unit tests; cluster duplicated subexpressions; test whether the newly formed library improves solving *new* tasks. Learn not just outputs, but reusable computational building blocks.

## Translation to JARVIS

Start with a restricted DSL, not unrestricted Python. A valid candidate skill is a pure function with deterministic semantics, dependencies, source provenance, complexity bound, and regeneration instructions. Store candidate metadata in the Grimoire lab. Let JARVIS read a concise reviewed **description**, but never auto-import generated source into its dispatcher.

## Experiment

**EXP-004.** Three arms: frozen baseline library; manually engineered library; automatically discovered/refactored library. Use independent training and withheld task families. Fix search budget and total cost; measure solved tasks, time to solution, number of reusable functions, complexity reduction, incorrect-function rate, and benefit after amortizing discovery cost. Repeat multiple seeds and guard against near duplicates in held-out tasks.

## Failure modes

Overfitting to training tasks, library bloat, accidental reliance on hidden state, invented abstractions that have no transfer value, and treating model confidence as proof. Search may cost vastly more than a later saved call; report lifetime break-even.

## Evidence and limitations

DreamCoder research demonstrates program-library learning on specific benchmark domains; it does not establish that JARVIS can recursively improve its general intelligence or weights.

## Sources

SRC-007.

## Promotion condition

Independent held-out transfer benefit, correct pure functions, reproducible compression, full discovery cost accounting and explicit human approval for inclusion in the callable skill library.



---


# GRM-008 — LILO: synthesize, compress, document

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

LILO combines program synthesis with library induction, compression and language-grounded documentation. A useful advance over a bare synthesized function is a reusable abstraction that can be described in words and rediscovered or invoked by later users/agents.

## Conventional assumption

Simply add every generated tool or script to JARVIS's memory. This grows code and retrieval cost without telling the agent which fragments generalize.

## Unconventional move

Define a structured **skill card** created from tested code: signature, exact behavior, preconditions, examples, cost, provenance, exclusions, known failure cases, and independent tests. Use compression to propose shared functions, and natural-language documentation for retrieval, but validate both against actual behavior.

## Translation to JARVIS

The Grimoire's research card format is the initial human-readable counterpart. Future library candidates can include a machine-readable callable manifest pointing to a reviewed binary/module, never a copied command string. Python remains an appropriate experiment environment; Rust ultimately controls registration and dispatch policy.

## Experiment

Part of **EXP-004**. Compare learned skill libraries with vs without documentation and retrieval; randomize descriptions to test whether language grounding improves accurate reuse. Evaluate unseen paraphrases, compositional generalization, compression ratio, answer accuracy, false retrieval and net compute.

## Failure modes

Documentation hallucinations, compressing distinct functions that only look similar, using examples that leak held-out tests, retrieval bias toward verbose descriptions, and accumulating inactive skills without lifecycle limits.

## Evidence and limitations

The LILO paper reports benefits on specific synthesis tasks; JARVIS benefit remains hypothetical. Stitch's compression is a specialized algorithm, not a general proof that shorter programs are always faster or smarter.

## Sources

SRC-008, SRC-016.

## Promotion condition

Each proposed function's docs agree with tests, retrieval points to correct library candidates under held-out paraphrases, and use reduces total costs without sacrificing task correctness.



---


# GRM-009 — STOKE stochastic superoptimization

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

STOKE searches for machine-code instruction sequences that are functionally equivalent to a target but optimize a cost such as performance. It explores a different space from high-level source rewriting, and can sometimes outperform mature compiler output on particular kernels.

## Conventional assumption

Whatever the compiler emits for a short kernel must already be globally optimal. Modern compilers are excellent, but instruction-level superoptimization is a valid research direction with its own tradeoffs.

## Unconventional move

Treat low-level optimization as **search subject to rigorous validation**, using a separate toolchain and experimental host. Start with tiny pure kernels and compare against optimized compiler baselines (`-O3`/LTO, relevant target). Account for CPU-specific instructions and microarchitecture effects.

## Translation to JARVIS

Not a voice-runtime component. It might later optimize a proven bottleneck in parsing, bitset operations or a binary codec. Because STOKE primarily targets x86-64 machine-code details and has environment/toolchain constraints, it is not the first experiment to run on the mixed Windows/Mac roadmap.

## Experiment

Deferred branch of **EXP-003** after a concrete native hotspot is identified. Use a read-only offline toolchain and isolation. Independently check equivalence across the declared machine model and adversarial edge cases. Benchmark across representative CPUs; measure compilation/search cost and portability.

## Failure modes

Undefined behavior, flags/stack calling convention differences, speculative execution changes, platform-specific speedups, microbenchmark noise and unsandboxed execution of generated native code.

## Evidence and limitations

The STOKE repository documents a real optimizer and published demonstrations, **not** guaranteed speedups for any new JARVIS workload. Do not run generated native code in the supervisor process.

## Sources

SRC-009.

## Promotion condition

Independent equivalence check, relevant hardware benchmarks, clear sustained gains and a conventional code-reviewed deployment/rollback process.



---


# GRM-010 — AlphaEvolve-style evolutionary algorithm search

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

AlphaEvolve illustrates a system in which language models propose algorithmic changes while automated evaluators test and score them. The critical element is not uncontrolled self-rewriting: it is **a bounded generate–evaluate–select loop** with trustworthy feedback.

## Conventional assumption

Ask one model to write an improved algorithm, trust its explanation, and repeat until it sounds clever.

## Unconventional move

Operate on pure function candidates in a restricted language. Candidate proposal can come from an LLM, mutation engine or random baseline. A separate evaluator checks validity and performance before a lineage is retained. Track compute costs, rejects and failure cascades.

## Translation to JARVIS

The initial research runner should be external to JARVIS and have no personal-memory access. Preserve a frozen baseline and multiple independent lineages. The candidate cannot modify acceptance tests, time limits or scoring rules. JARVIS gets a concise verified report, not authority to push a candidate live.

## Experiment

**EXP-005.** Compare equal-budget random mutations, deterministic heuristics and LLM-proposed variants on a small pure algorithm benchmark. Use protected held-out tasks, strict candidate timeouts and identical total CPU/GPU budgets. Score successful independent lineages rather than cherry-picked best candidates.

## Failure modes

Goodharting the evaluator, leaked benchmarks, mutation of the grader, expensive search disguised as cheap inference, and choosing a best-of-many candidate without correcting for selection effects. Candidate safety and end-to-end cost matter more than attractive isolated scores.

## Evidence and limitations

Google DeepMind's published work reports algorithm discoveries under specific evaluators. This is not evidence of endless recursive intelligence growth or autonomous safe self-modification in JARVIS.

## Sources

SRC-010.

## Promotion condition

Independent evaluator isolation, predeclared budget/statistics, reproducible held-out gains, an audit trail, rollback and human approval.



---


# GRM-011 — Formal proof and solver-aided programming

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

Lean 4 provides a proof assistant and functional language; Rosette offers solver-aided program synthesis and verification. These tools convert selected correctness claims into formal or solver-checkable obligations instead of asking an LLM to reassure us.

## Conventional assumption

If a generated refactor passes unit tests, it must be semantically equivalent everywhere. Tests sample behavior; they usually cannot establish universal correctness over an unbounded input space.

## Unconventional move

Represent selected invariants as proof obligations. Examples: two total pure functions agree for all `u8` inputs (exhaustive), a transformation preserves an algebraic invariant under explicit assumptions, or a schedule meets all finite resource constraints. Use formal proof where the model is small enough; use randomized tests otherwise and label the strength of evidence correctly.

## Translation to JARVIS

Place proof and solver tooling in the separate experiment lab. Require that a particular rewrite or compiler-generated candidate cannot be promoted without a proof or a documented finite-domain exhaustive check. Rust can verify a result artifact's provenance but cannot treat an arbitrary claimed `proof: true` string as proof.

## Experiment

A support module for **EXP-003**. Compare proof/solver overhead to ordinary random tests on a small rewrite corpus; seed known incorrect rewrites and require the checker to reject them. Inspect solver timeouts and unknown results rather than interpreting them as success.

## Failure modes

Incorrect specification, unverified axioms, proof artifacts tied to different source revisions, solver unknown/timeouts, trusting autogenerated proof comments and confusing formal properties with real-world intent.

## Evidence and limitations

Formal methods can prove properties of **the encoded model**; they do not establish that the model captures every security or user requirement.

## Sources

SRC-011, SRC-012.

## Promotion condition

Machine-checkable property with pinned source/assumptions and independently rerunnable checker, plus separate security review of unmodeled behavior.



---


# GRM-012 — Datalog and derivation-aware reasoning

**Marginalia:** This card is a research interpretation, not permission to execute, mutate a model, or change an evaluator.

## Principle

Datalog-style rules infer facts from a finite database of relations. egglog combines logic-programming concepts with e-graph mechanisms. Rather than storing every inferred conclusion as a free-form memory, a symbolic system can track **which source facts and rules justify it**.

## Conventional assumption

Treat every generated summary sentence as another durable fact. Repeating this produces untraceable contradictions and stale derived conclusions.

## Unconventional move

Store base facts separately from derivations. For a tiny research example: an experiment can be `eligible_for_review` only if a preregistration is frozen, tests pass, and an independent report exists. This derives *review eligibility*, **not** capability approval or permission to act.

## Translation to JARVIS

Keep the research provenance graph distinct from the existing user-approved Knowledge graph. Use Datalog for read-only queries about dependencies, source claims, missing evidence and contradictory research status. Do not let a fact like `approved_by_model(X)` satisfy policy; human approval is an external authenticated event in Rust.

## Experiment

Prototype on 100–1000 synthetic records with deliberate contradictions, deleted source facts, cycles and stale evidence. Compare logic-derived reports to an independent reference implementation. Measure query latency, incremental recomputation cost and invalidation accuracy.

## Failure modes

Inferred facts without provenance, stale caches, cyclic rule semantics, confusing open-world and closed-world assumptions, nonmonotonic negation issues, and treating inferred eligibility as authorization.

## Evidence and limitations

Datalog is well-established for rules; applying it to real-world personal memory or scientific truth requires robust source and uncertainty modeling. This entry is a read-only complement, not a replacement for existing Obsidian review.

## Sources

SRC-006, SRC-013.

## Promotion condition

Deterministic derivations, source-level explanations, correct invalidation and a strict prohibition on inferred permission grants.



---


# GRM-013 — Stitch corpus-guided abstraction compression

**Marginalia:** The best first experimental component may be the *compressor*, not a large learned model.

## Principle

Stitch implements top-down, corpus-guided synthesis of reusable lambda abstractions that compress collections of programs. Unlike the full DreamCoder wake/sleep pipeline, it can be applied to an existing corpus of small programs. The POPL 2023 research reports substantially reduced resource consumption and search time **for its tested library-learning benchmarks**. LILO later used Stitch as a core compression component.

## Conventional assumption

You must train, fine-tune, or enlarge an LLM before it can acquire reusable knowledge. But reusable computational abstractions can be discovered outside neural weights by analyzing a corpus of previously solved tasks.

## Unconventional move

Provide a corpus of compact, typed DSL programs to an abstraction compressor; receive candidate shared functions and rewrites; verify that substituting each candidate preserves behavior; then test whether the reduced library improves held-out search and retrieval. The compressor is an **offline lab** component, not a production interpreter.

## Translation to JARVIS

Use Rust/Python Stitch tooling only if the tiny library experiment has enough repeated program structure for compression to be meaningful. Because JARVIS already has Python for experiments and Rust for supervision, Stitch's existing Python bindings and Rust implementation are a closer practical match than rebuilding DreamCoder end-to-end. A successful abstraction becomes a candidate skill card, never a new runtime action by itself.

## Experiment

**EXP-004**, add a distinct candidate arm: (1) frozen base library, (2) human compression, (3) simple suffix/tree-subexpression compressor, and (4) Stitch. Freeze identical training programs and separate held-out task grammars. Compare compressed corpus description length **including the library**, rewrite semantic correctness, discovery CPU/memory cost, number of independently reusable functions and held-out solve benefit. The reported speedups in Stitch's paper are prior literature evidence, **not** projected JARVIS speedups.

## Failure modes

Shortened syntax with no end-to-end runtime benefit; abstractions that fit only the training corpus; binding/capture mistakes when rewriting lambda terms; a library larger than the saved corpus; dependencies and offline packaging of third-party Rust/Python code. Research dependencies must not be imported into the voice process.

## Evidence and limitations

The POPL paper and reproducibility artifact establish the tool and its studied benchmarks; LILO is evidence that it composes with language-guided library learning. No transfer result has yet been measured for this JARVIS system. The workload must be carefully represented as the DSL format Stitch accepts.

## Sources

SRC-016, SRC-008.

## Promotion condition

Independent semantic equivalence checks, repeatable compression under a frozen corpus, demonstrable held-out transfer or measurable reuse, total cost accounting and a human-reviewed skill manifest. No auto-execution of compressed programs.



---

# Appendix B — Research experiment protocols



---


# EXP-001 — Typed task DSL and bottom-up reuse

**Status: a proposed protocol, not a completed run.** Budget caps, seeds, data revisions, hardware details, statistical test and effect sizes require freezing in a per-run preregistration before any confirmatory claims. The numeric thresholds below are *proposed pilot decision thresholds*, not established results.

## Research question

Can a deliberately bounded, typed plan language reduce repeated model generation and improve reliability across ordinary JARVIS **read-only** tasks?

## Hypothesis and null

H1: A compositional plan representation beats an equivalent fixed-functions baseline on total time-to-correct-answer or maintainability for repeated diverse tasks. H0: It adds parser/runtime overhead, no correctness gain, or any benefit is merely eliminated prompt size.

## Frozen inputs and arms

Before execution freeze >=24 synthetic read-only fixtures split by *task family* into practice and held-out families. Include near-duplicate paraphrases only in practice. A: ordinary hand-written Python functions called by deterministic router. B: declarative JSON AST compiled into the same functions. C: LLM plan proposal followed by identical AST validator. If LLM availability/latency dominates, report A/B separately and label C a later run. Fixed candidate budgets, prompts and hardware manifest.

## Technical contract

Allowed operators at pilot: `literal`, `select_approved_mock`, `filter`, `map_bounded`, `summarize_template`, `present_draft`. Reject filesystem/network/process operators, unbounded loops, recursion, untyped closures, unknown JSON fields, max depth > 8 and total node count > 32. The mock is synthetic; no personal Knowledge or live tools are read. Do not conflate this mock with the existing user memory gate.

## Measures

Exact fixture correctness, parse/rejection rate, AST bytes and tokens, per-arm wall-time and CPU-seconds, number of LLM calls, repeated-task reuse fraction, false-permission attempts accepted (must be zero), and 95th percentile across repetitions. Record cold/warm separately. Use per-task pairing and fix all policy checks.

## Falsification / go–no-go

**Gate PASS:** 100% fixture equivalence, all policy-adversarial fixtures rejected, bounded execution, and at least one predeclared utility metric better without >5% p95 latency regression (pilot threshold, not a claimed result). **NO-GO:** any authorization bypass or non-equivalence. Otherwise INCONCLUSIVE and report result with no promotion.

## Control and artifacts

Frozen fixture manifest/hash; run manifest with Git SHA, OS, CPU, Python/Rust versions, seed and operator-version hash; per-case result JSONL and grouped report. Never report the DSL as being “smarter” solely from compressed syntax.

## Integration path

Only after independent review may a read-only AST validator be placed behind disabled `grimoire` flag in `jarvis-core`. No executable DSL nodes are added to policy in this gate.



---


# EXP-002 — Bitset representation vs ordinary data structures

**Status: a proposed protocol, not a completed run.** Budget caps, seeds, data revisions, hardware details, statistical test and effect sizes require freezing in a per-run preregistration before any confirmatory claims. The numeric thresholds below are *proposed pilot decision thresholds*, not established results.

## Research question

Are compact bitsets a measurable net advantage over sets, sorted arrays or dictionaries for the actual small-universe operations JARVIS uses?

## Hypothesis and null

H1: For known finite universes and repeated intersection/membership, bitset representation reduces steady-state allocations/latency. H0: conversion, sparsity, updates and cache behavior cancel the benefit.

## Frozen inputs and arms

Synthetic universes of 32, 64, 256, 4096 and 65536 IDs; densities of 1%, 10%, 50% and 90%; randomized plus clustered distributions. A: Python/Rust hash set. B: sorted vector + binary search. C: `u64`/`Vec<u64>` bitset. Optional D: compressed bitmap only after a separate declared dependency. Keep all arms semantically identical for union, intersection, subset, insert/delete and serialization.

## Technical contract

Use unsigned checked indexing. Explicitly handle empty universe, out-of-range IDs, duplicate IDs, serialization layout, large shifts and 0/64-boundary conditions. Never substitute bitset masks for JARVIS's existing typed permission checks.

## Measures

Allocation count if available, peak resident memory, serialized bytes, throughput, median and p95 latency for 30+ warmed repetitions, setup/conversion costs, CPU-seconds and correctness mismatches. Report speedup by density and operation, not a blended winner score.

## Falsification / go–no-go

**PASS:** full correctness for edge fixtures + randomized regression and at least 10% p50 end-to-end improvement on a selected real read-only hotspot without >5% p95 regression or memory regression. **NO-GO:** mismatch, index hazard, or no actual hot path. No-gain results are scientifically useful.

## Control and artifacts

Input generator seed/hash, source revisions/compiler flags, per-operation CSV/JSONL, raw timing output, CPU frequency/thermal notes and architecture summary. Use hyperfine or Criterion only after dependency approval; Python `time.perf_counter` is a pilot convenience, not a substitute for rigorous benchmark harnessing.

## Integration path

Prefer ordinary reviewed Rust data-structure change to `jarvis-core` if it demonstrates a specific improvement. Do not use self-modifying low-level code.



---


# EXP-003 — Equality saturation vs basic optimization

**Status: a proposed protocol, not a completed run.** Budget caps, seeds, data revisions, hardware details, statistical test and effect sizes require freezing in a per-run preregistration before any confirmatory claims. The numeric thresholds below are *proposed pilot decision thresholds*, not established results.

## Research question

Can an e-graph-based optimizer discover cheaper equivalent expressions in a deliberately restricted JARVIS-like DSL without exploding search or corrupting semantics?

## Hypothesis and null

H1: equality saturation extracts better expressions than deterministic canonical simplification under a fixed budget. H0: equivalent candidates are already found by ordinary simplification or extraction overhead dominates savings.

## Frozen inputs and arms

Generate typed ASTs from `u8` literals and variables with wrapping `+`, `-`, `*`, `&`, `|`, `^`, `!`, and shifts with explicitly constrained counts. Freeze operation semantics, 100+ expressions, AST generator seed, five predeclared sound rewrite rules, and cost model **before** testing. A: simple hand-written rewrite pass. B: egg/egglog-style e-graph saturation capped at fixed node and time budgets. C: a disabled, model-suggested rewrite branch only after source and separate independent soundness review.

## Technical contract

Exhaustively compare every `u8` input for one-variable functions (256 cases), every pair only for a small predeclared subset (65,536 each), plus held-out randomized multi-variable vectors. Add adversarial rules `x*0=0` on floats (must reject if floats ever supported), non-total division, signed overflow, shift semantics, and side-effect examples. Prohibit host-language eval of generated expression text. Use interpreter or formally specified AST evaluator only.

## Measures

Number of proven/evaluated equivalent candidates, extracted AST cost, full end-to-end optimization time, AST/interpreter run time, memory peak/e-graph size, timeout fraction and correctness failures. Report the amortization break-even for repeatedly invoked expressions.

## Falsification / go–no-go

**PASS:** 0 wrong extracted candidates across frozen exhaustive+random tests, bounded saturation, and a preregistered gain in candidate cost *and* relevant repeated runtime after accounting for optimizer time. **NO-GO:** any semantic mismatch or unbounded resource use. **INCONCLUSIVE:** improvement only in AST size but not actual performance.

## Control and artifacts

Exact operator spec, rewrite set with proofs/tests, generated task corpus, input hashes, compiler/version manifest, serialized e-graph stats, extracted ASTs and held-out reports. STOKE/x86-64 native optimization is *not* required here; treat it as a later subprotocol with independent safeguards.

## Integration path

Optional specialized research library only. Approved optimization artifacts are reviewed code changes, not runtime-swapped machine code.



---


# EXP-004 — DreamCoder/LILO/Stitch transferable skill library

**Status: a proposed protocol, not a completed run.** Budget caps, seeds, data revisions, hardware details, statistical test and effect sizes require freezing in a per-run preregistration before any confirmatory claims. The numeric thresholds below are *proposed pilot decision thresholds*, not established results.

## Research question

Can JARVIS learn a compact, human-readable library of pure operations that transfers to held-out **task families** and reduces repeat inference or search cost?

## Hypothesis and null

H1: learned abstractions increase held-out solve rate or reduce cumulative work vs a frozen library. H0: benefits vanish when leakage, discovery compute and library overhead are included.

## Frozen inputs and arms

Choose an exact tiny DSL first (string cleaning, list transforms or integer expression trees). Predefine at least 3 disjoint *structural* task families for practice and >=2 disjoint held-out families; prevent near-duplicate generated questions across splits with grammar-level exclusions. Freeze search budget/seed. A: base library frozen. B: human-curated abstractions from practice tasks. C: automatically extracted/compressed abstractions (a minimal local implementation first; optional Stitch after offline dependency review). D: learned library + validated natural-language descriptions/retrieval. DreamCoder/LILO full reproduction is separate and not assumed.

## Technical contract

Each candidate pure function has declared input/output types, bounded operations, deterministic evaluator, provenance, positive and negative test cases, dependence graph, documentation and artifact hash. Keep candidate code out of JARVIS's runtime. A validator can reject program rewrites that change outputs before accepting any compressed library. Generated documentation cannot be used as a test oracle.

## Measures

Held-out solve rate per family, description length including library overhead, time per solve, total discovery CPU/GPU seconds, number of reusable functions, false retrieval, function-semantic error rate, correction burden, break-even number of uses. Statistical unit is task family and independent seed, not cherry-picked candidate solutions.

## Falsification / go–no-go

**PASS:** no incorrect promoted functions, stable gains across >=2 held-out families and >=3 independent seeds, and better total cost at a predeclared usage horizon. **NO-GO:** test leakage, unreproducible abstractions, library growth with no transfer, or worse cumulative cost. **INCONCLUSIVE:** improvement only on training-like paraphrases.

## Control and artifacts

Corpus generator/grammar and split SHA, fixed prompts, program AST library JSON with provenance/hashes, per-task solve trace, independent grader checks, sensitivity analysis and honest failure report. Publish positive and negative cases.

## Integration path

First import just reviewed **documentation** via bounded read-only Grimoire retrieval. Actual function execution remains a separately reviewed Rust capability with explicit contracts and authorization. Do not equate learning new library routines with Horizon's weight self-editing claim ladder.



---


# EXP-005 — Independent evaluator guided algorithm evolution

**Status: a proposed protocol, not a completed run.** Budget caps, seeds, data revisions, hardware details, statistical test and effect sizes require freezing in a per-run preregistration before any confirmatory claims. The numeric thresholds below are *proposed pilot decision thresholds*, not established results.

## Research question

Do LLM-suggested algorithm mutations outperform equally budgeted random/heuristic search on a pure benchmark when the evaluator is independent and compute is honestly counted?

## Hypothesis and null

H1: LLM guidance improves quality of valid independently tested candidates per compute-dollar/CPU-second. H0: gains come from candidate volume, leaked tests or selection bias, and disappear under controlled budgets.

## Frozen inputs and arms

A: random syntactically valid AST mutations; B: deterministic targeted rewrite heuristic; C: LLM-proposed bounded AST mutations. Same starting programs, candidate count, wall-clock and compute budgets (report both even if cannot match perfectly), identical public feedback, five independent lineages per arm at pilot scale. Hidden evaluator separated by OS permissions, not just a folder. Freeze objective, grammar, sampling and seed generation before launch.

## Technical contract

Candidates can only submit typed ASTs to the runner (never grader code, shell commands, paths, arbitrary modules or test questions). Reject overly large or invalid programs, side effects, infinite loops, resource overruns, and attempts to modify the evaluator. An isolated worker enforces CPU/memory/time budgets; run on synthetic data only. JARVIS gets reports, not execution privileges.

## Measures

Validity rate, independent held-out correctness, speed/cost improvement over frozen baseline, total proposer inference GPU-seconds plus evaluation CPU-seconds, number of candidates, effect by lineage, regression retention, attempts with invalid output, user review cost, and power where reliable.

## Falsification / go–no-go

**PASS:** all candidate and grader trust checks pass; stable improvement across multiple independent lineages and held-out families; total cost advantage vs randomized controls. **NO-GO:** grader leakage, unrestricted code execution, semantic failures or uncontrolled budget. **INCONCLUSIVE:** a single exceptional best candidate with no lineage-level repeatability.

## Control and artifacts

Immutable evaluator version + hashes, frozen seed list and corpus, allowed mutation grammar, resource limits, isolated access-control test, append-only per-candidate status and hash, cumulative compute report, and explicit selection-bias caveat.

## Integration path

Do not connect this to recursive weight editing directly. A later preregistration can test whether the skill library improves *proposal quality*, distinct from whether a successor model improves its **own** editor ability in Horizon.



---

# Appendix C — Architecture and integration



---

# ADR-GR-001 — Research knowledge in Markdown; execution authority in existing Rust core

**Status:** Proposed for review  |  **Date:** 2026-11-23

## Context

JARVIS already has a Python 3.11 intelligence/UI layer, a reviewed Obsidian-compatible Markdown memory vault, and `jarvis-core/` (Rust) for typed decisions, queued obligations, operational journaling and policy. The `Horizon/` folder defines preregistered F/W/B research. The Grimoire must not become a parallel assistant, a hidden agent, or an unreviewed memory write path.

## Decision

1. Keep this Grimoire as Git-versioned Markdown outside the ignored personal memory vault. Technical sources and generated proposals are always read as **research data** with explicit provenance.
2. Use strict flat YAML metadata in each card and produce a canonical, deterministic JSON index for Rust to ingest (only after trusted commit/hash pinning and human review). Avoid a second permanent index format or vector database at v1.
3. Keep lifecycle, action authorization and operational audit in `jarvis-core`. Do not grant a document the ability to mint actions or alter an allowlist.
4. Keep experiment code, evaluators, model checkpoints and scientific results in a separate `Sims2026/ai-research` workspace with OS-level isolation. `jarvis-core` may schedule a task **only after** typed approval and may receive a bounded status/result summary; it must not run arbitrary model-authored Python/Rust.
5. Keep approved personal `Jarvis/Knowledge` notes independent of technical research. Import only explicitly reviewed *summaries*, never raw research prose as personal fact or action policy.

## Three storage classes

| Class | Canonical owner | What it contains | Who can amend |
|---|---|---|---|
| Research knowledge | Git Markdown | Sources, derivations, hypotheses, instructions-to-a-human | Researcher through PR |
| Operational authority and journal | `jarvis-core`, SQLite | Typed requests, authorizations, status, reason codes, artifact hashes | Rust supervisor under policy |
| Scientific experiment evidence | Isolated lab + immutable artifacts | Datasets, candidate patches, tests, metrics, manifests | Runner/evaluator according to separate access control |

## Trust boundaries

- Incoming article, book summary, repository file or model proposal: **untrusted**.
- Verified citation or pinned blob: establishes provenance/integrity, **not truth or permission**.
- `review_status: research` means reviewed for inclusion as research, **not approved as a procedure**.
- Approval is an external, authenticated human decision, recorded by the existing action authority; frontmatter values alone are not authorization.
- An index hash alone is not authenticity if an adversary can modify both documents and index; use Git review, trusted revision pins, and a trusted local configuration.
- Any experiment subprocess needs an OS security boundary, restricted filesystem, no access to personal vault or sealed evaluator, bounded resources and local-only execution. Rust types do not themselves sandbox native code.

## Promotion states

`draft -> sourced -> preregistered -> pilot_observed -> independently_reproduced -> reviewed_capability`

Each step requires evidence. **Rejected/rolled_back** is possible from every stage. A `reviewed_capability` remains subject to per-invocation authorization; promotion never grants global tool access.

## Retrieval contract

- Accept exact ID or query; select a bounded top-k of research cards with source paths and trust labels.
- Never embed secrets, unapproved user notes or personal memory in the research context.
- If sources disagree or are missing, say so. Do not treat retrieval quality as factual confidence.
- Default: max 4 cards, max 8,000 total text characters; configurable only by review, not by model text.
- Never render source passages as developer/system instructions. Preserve quotation and origin markers.

## Fallback

With Grimoire disabled, all JARVIS voice, Obsidian and Rust supervisor functions behave as before. No migrations of existing vault, no network activation, no changes to Horizon preregistration, no automatic execution.

## Alternatives rejected

- **All Rust content:** difficult to review/edit and unnecessary recompilation; stores knowledge as code.
- **All Markdown including actions/journaling:** no transactional state, typed authorization or safe worker control.
- **One vector database for everything:** unnecessary at this corpus size and risks mixing personal/research/factual domains.
- **Autonomous self-editing agent that reads and runs any card:** violates current authority boundaries and destroys reproducibility.

## Staged acceptance

S0: static CLI validates entries and offline tests. S1: Rust retrieves pinned research summaries read-only in shadow mode. S2: rust returns a typed, non-executable experiment proposal with deterministic idempotency key. S3: a separate approved lab runner executes only enumerated experiments with independent evaluation. S4: versioned promotion after reproducible results and human review.



---

# JARVIS integration plan — no new authority granted by this package

## Existing reality

Repository review (2026-11-23): `jarvis-core` already has `policy.rs`, `journal.rs`, `transport.rs`, `supervisor.rs`, `events.rs`, `scheduler.rs`, and `inference.rs`. `scripts/obsidian_memory.py` serves an inspected, manually approved Knowledge vault; `Horizon/ROADMAP_v1.2.md` and `Horizon/preregistration.v1.2.yaml` govern the separate F/W/B research program. **Preserve these paths.**

## Stage 0: file-only

Keep the Grimoire as separate versioned Markdown and run `python tools/grimoire.py validate`. Export `index.generated.json` only as a transient artifact. Never give this directory the status of an approved user memory or a tool registry.

## Stage 1: read-only Rust adapter, shadow mode

Add `jarvis-core/src/grimoire.rs` (proposed; not included/compiled here), behind `[grimoire] enabled = false, retrieval_only = true`. Parse a trusted locally pinned `index.generated.json` via existing `serde_json`; resolve approved local path relative to configured root; canonicalize/restrict to root; verify SHA-256 against a reviewed manifest (adding a hash crate requires approval/vendor). Build a bounded `ResearchExcerpt` result with `id`, `title`, `origin`, `research_status`, `provenance`, `text`, `revision`, `integrity_hash`. Never import parsed Markdown into capability registration.

**Important:** indexing in Python is just packaging. At runtime Rust should either revalidate content against the pinned reviewed index or own the read-only indexing. Merely checking `sha256(index)` generated by the same untrusted files is insufficient.

## Stage 2: typed proposals over existing local IPC

Illustrative wire message (documentation, not an implemented operation):

```json
{
  "version": 1,
  "kind": "request",
  "request_id": "grm-demo-001",
  "op": "grimoire_propose_experiment",
  "body": {
    "entry_id": "GRM-006",
    "experiment_id": "EXP-003",
    "mode": "dry_run",
    "revision_pin": "TRUSTED_REVIEWED_GIT_SHA"
  }
}
```

The implemented Rust wire format and `op` enum must be confirmed against `transport.rs` before creating actual code; this is a **desired logical contract**, not a verified compatible frame. Reject unknown operations and fields. Reply with a proposal reference, prerequisites, estimated budget, authorization state and manifest digest; never a raw shell command to execute.

## Stage 3: lab bridge — never inside voice turn

A human-approved runner in a separately permissioned Windows account/VM consumes an allowlisted experiment ID and a frozen manifest. Offline provisioning occurs at build time only. It executes on frozen public/synthetic practice data, writes content-addressed results and returns a small signed/verified report or a path/hash JARVIS can read. The grader and sealed evaluation data must be inaccessible to candidate generation.

## Stage 4: review and capability promotion

Require: tests pass; independent evaluator attests semantic correctness; repeated held-out results; measured total compute; source/security review; opt-in integration PR; rollback path. `policy.rs` still decides per-request permissions. New capability registration is a code/review operation, never the outcome of a Markdown tag.

## Data flow

```text
question -> model intent candidate -> typed read-only research query
  -> pin/version/trust-boundary check -> bounded excerpts -> explanatory response

experiment suggestion -> typed proposal -> human review -> isolated runner
  -> independent evaluator -> immutable report -> reviewer -> capability PR
  -> Rust policy engine per-invocation authorization
```

## Proposed Rust interface (types, not working code)

```rust
struct ResearchQuery { query: String, limit: u8, max_chars: u32 }
struct ResearchExcerpt {
    id: String,
    title: String,
    revision: String,
    source_ids: Vec<String>,
    trust_label: String, // untrusted_research_data
    content: String,
}
struct ExperimentProposal {
    entry_id: String,
    experiment_id: String,
    revision_pin: String,
    budget_request: ResourceBudget,
    required_review: bool,
}
```

The real code should reuse existing event/policy/journal types rather than duplicating them. Do **not** write an ad hoc executor into `jarvis-core`.

## Local/offline and hardware

Keep CPU-small proof tasks in Sims2026 first. The RTX A4500 GPU is reserved for research stages that actually need it. Do not alter the frozen Qwen3.5 BF16 weight-edit protocol, fallback-kernel measurements, or preregistration due to Grimoire discoveries. If a discovered optimization touches it, define an additional study arm or amendment and independently retest.

## Ready-to-assign Devin task

Implement Stage 1 on a feature branch after reviewing this volume: static reviewed index loader; strict path containment and digest checks; max-k/char caps; unit tests for path traversal, malformed frontmatter, rejected overrides, missing/modified sources and disabled flag; shadow-mode integration test; zero changes to authorization. Open PR with test output and rollback instructions. **Do not implement Stage 2/3 in the same PR.**



---

# Appendix D — Source ledger

# Primary sources, code and caveats

This source ledger links to public primary sources and official code. A source supports the cited mechanism **in its studied scope**; it does not validate our proposed JARVIS application. All cards are independent original commentary, not copied book chapters. Vendor any third-party code only after license and version review.

### SRC-001 — Let Over Lambda — Doug Hoyte

- **Type:** book
- **Link:** https://letoverlambda.com/index.cl
- **Scope:** Macro programming and language-building; book free online.

### SRC-002 — On Lisp — Paul Graham

- **Type:** book
- **Link:** https://paulgraham.com/onlisp.html
- **Scope:** Bottom-up and higher-order programming; official free online copy.

### SRC-003 — Hacker's Delight (Second Edition) — Henry S. Warren Jr.

- **Type:** book
- **Link:** https://www.pearson.com/en-us/subject-catalog/p/hacker-s-delight/P200000000672/9780321842688
- **Scope:** Bit tricks and integer algorithms; publisher page, no reproduction of book.

### SRC-004 — The Art of the Metaobject Protocol — Kiczales, des Rivières, Bobrow

- **Type:** book
- **Link:** https://mitpress.mit.edu/9780262610742/the-art-of-the-metaobject-protocol/
- **Scope:** Reflective system design and metaobject protocols.

### SRC-005 — miniKanren project and relational programming

- **Type:** project
- **Link:** https://minikanren.org/
- **Scope:** Relational programming examples and research.

### SRC-006 — egglog (e-graphs + Datalog)

- **Type:** code
- **Link:** https://github.com/egraphs-good/egglog
- **Scope:** Rust equality saturation / relation library.

### SRC-007 — DreamCoder — Ellis et al. (PLDI 2021)

- **Type:** paper_and_code
- **Link:** https://github.com/ellisk42/ec
- **Scope:** Wake/sleep program synthesis and learned libraries; paper https://doi.org/10.1145/3453483.3454080

### SRC-008 — LILO — Grand et al. (ICLR 2024)

- **Type:** paper_and_code
- **Link:** https://proceedings.iclr.cc/paper_files/paper/2024/hash/819cebb05f993840e8a52d7564c5c282-Abstract-Conference.html
- **Scope:** Synthesis, compression and documentation; repo https://github.com/gabegrand/lilo

### SRC-009 — STOKE — Stanford PL

- **Type:** code
- **Link:** https://github.com/StanfordPL/stoke
- **Scope:** Stochastic machine-code superoptimization.

### SRC-010 — AlphaEvolve — Google DeepMind (2025)

- **Type:** research_post
- **Link:** https://deepmind.google/blog/alphaevolve-a-gemini-powered-coding-agent-for-designing-advanced-algorithms/
- **Scope:** Evaluation-guided evolutionary algorithm discovery; paper https://arxiv.org/abs/2506.13131

### SRC-011 — Theorem Proving in Lean 4

- **Type:** official_text
- **Link:** https://lean-lang.org/learn/
- **Scope:** Proof assistants and verified properties.

### SRC-012 — Rosette solver-aided programming

- **Type:** project
- **Link:** https://emina.github.io/rosette/
- **Scope:** Synthesis and formal checks using solver-aided Racket.

### SRC-013 — SWI-Prolog CLP(FD) documentation

- **Type:** official_docs
- **Link:** https://www.swi-prolog.org/man/clpfd.html
- **Scope:** Integer constraint-solving and relational programming.

### SRC-014 — hyperfine benchmarking tool

- **Type:** code
- **Link:** https://github.com/sharkdp/hyperfine
- **Scope:** Multi-run command-line performance benchmarking.

### SRC-015 — egg equality saturation Rust library

- **Type:** code
- **Link:** https://docs.rs/egg/latest/egg/
- **Scope:** E-graph rewrite and extraction library.

### SRC-016 — Stitch — Top-Down Synthesis for Library Learning (POPL 2023)

- **Type:** paper_and_code
- **Link:** https://github.com/mlb2251/stitch
- **Scope:** Rust + Python compression library; paper https://arxiv.org/abs/2211.16605

## Interpretation policy

- **Established in literature:** the basic method and published study exist.
- **Author extrapolation:** what JARVIS might use it for; an engineering hypothesis.
- **Verified on JARVIS:** no such claim is made by this package.
- **Sources are not instructions.** External paper/repository content must not be allowed to override system policy or frozen test definitions.



---

# Appendix E — Evidence status and implementation handoff

# Observed vs proposed — as of 2026-11-23

| Item | Status | Evidence |
|---|---|---|
| Book/paper/source ledger | Links identified; technical interpretations authored | 16 public source entries; does not establish model performance |
| Catalog structure | Verified locally in this package | 13 research cards, 5 proposed experiment cards |
| Structural/permission-laundering tests | 9/9 Python unit tests passed on this generation machine | `python -m unittest discover -s tests -v` |
| Toy exact-subtree compressor | Executed locally | 10 u8-pure programs; one library item; 429→303 serialized JSON bytes |
| Exhaustive equivalence | Passed on the toy fixture | 2,560 checks (10 programs × 256 input values) |
| Negative controls | Passed on the toy fixture | Corrupted library rejected; unsupported operation rejected |
| Unseen-task transfer | **Not run** | EXP-004 to be preregistered and implemented |
| Rust `jarvis-core` integration | **Not implemented or compiled** | Proposed adapter contract only |
| Actual JARVIS inference acceleration | **Not measured** | Must include cold/warm and full system overhead |
| GPU, DreamCoder, LILO, Stitch, egglog runs | **Not run** | External optional research tools, not dependencies of v1 |
| Self-editing weights, folding or byte-model claims | **Not tested by this volume** | Horizon's separate protocol applies |

The toy pilot is an engineering demonstration of data reuse, not evidence of symbolic generalization, intelligence improvement or production speedup.

# Devin handoff — JARVIS Grimoire, Stage 0 → Stage 1

**Objective:** Review and integrate this *research-only* Grimoire into `drdre72/JarvisProject` without altering the application's authority model, voice path, approved personal-memory logic or Horizon preregistrations.

**Inspected repository commits (2026-11-23):**
- JarvisProject/main: `49b8adf962a650238785e702a2d65fe6ec25b837`
- Sims2026/main: `16fb91159fedd817aca09b930855fe212a6657ed`

**Do not blindly cherry-pick based on these SHA values if main advances.** Reinspect current branch and reconcile.

## Priority 0 — Read and audit

Read `README.md`, `GRIMOIRE.md`, `architecture/ADR-GR-001.md`, `architecture/INTEGRATION.md`, all EXP cards, and existing `JarvisProject/AGENTS.md`, `OBSIDIAN_BRAIN.md`, `docs/decisions/ADR-0006-rust-supervisory-runtime.md`, `Horizon/ROADMAP_v1.2.md`. Validate source links, licenses and duplicate conceptual coverage. The 2026-11-23 package is a **proposal**, not a merged change.

## PR 1 — Static Grimoire adoption

On a feature branch (not main), place this tree under `grimoire/`. Do not auto-sync entries to `memory/obsidian-vault/` or approved Knowledge. Run:

```powershell
py -3.11 grimoire/tools/grimoire.py validate
py -3.11 -m unittest discover -s grimoire/tests -v
py -3.11 grimoire/tools/abstraction_pilot.py
```

Expected for this artifact: 13 entries, 5 experiments, 16 sources, 9 Python unit tests; toy pilot 2,560 exhaustive checks, 429→303 JSON bytes. Rerun rather than trusting these output values. Review every card for claim overreach. `index.generated.json` can be created and commit-pinned in the PR only after all source/card changes are frozen. Include all hash manifests in PR.

**Acceptance:** no runtime code changes; no new dependencies; no network usage during checks; no personal memory or Horizon modification; static index reproducible; strong warnings retained.

## PR 2 — Read-only Rust adapter only (after PR 1 review)

Implement `jarvis-core/src/grimoire.rs` consuming the reviewed/pinned static index using existing `serde_json`, with disabled feature flag and shadow-only retrieval. Reuse `policy.rs` and `transport.rs` types after inspecting their exact structures. **Do not add an IPC op until confirming the existing frame schema and testing wire compatibility.**

Tests: disabled-default unchanged; path traversal/symlink refusal; changed file/hash refusal; missing source; duplicate IDs; size/depth caps; bounded exact-ID and keyword retrieval; malicious markdown treated as quoted data; attempts to set `review_status: approved` not permission-granting. Provide a rollback to disabled behavior.

**Acceptance:** no action dispatch, no auto-registration, no vault writes, no process spawn, no network, no grader code. Rust core still passes its native test suite and existing Python protocols.

## PR 3 — Separate lab pilot, not a JARVIS runtime feature

Add `Sims2026/ai-research/grimoire-experiments` only after Horizon's existing research environment has been provisioned. Freeze the EXP-001 DSL and EXP-004 skill-compression protocols. Start with pure typed ASTs and a naive compressor, then compare `Stitch` as a **separately pinned optional dependency**. No need to install the whole DreamCoder stack. In the evaluator, use independently held-out task families, compare full library overhead and actual compute, and count negative results.

**Acceptance:** independent evaluator/test isolation demonstrable; reproductions attached; no modifications to Gate 0/0.5/1 W, F or B arms; no production capability promoted; no unsupported performance claims.

## Mandatory reporting

At each PR, report exact changed paths, added dependencies, commands executed, pass/fail counts, source versions, untested environments, open risks and rollback instructions. Keep the evidence chain. If a proposed improvement is only a hypothesis, say so. Do not mark a gate as proven from mocks alone.
