# The Observation Channel Is the Bottleneck — a research note (v2)

*Gate 1.2 post-campaign note for the Frontier Synthesis track (FS-05, representation specificity). Companion to `ai-research/research/gate1_2_campaign_report.md`. Status: exploratory inference, not a protocol change. v2 incorporates reviewer corrections (attributions, causal-abstraction controls, compression economics, three-level channel scheme, locality formalization).*

## 1. The asymmetry the campaign measured

Two numbers from Gate 1.2 belong together:

- **Collecting** functional observations across 263 hooked modules: **~6 s**, ~8.6 GB peak — cheap, mechanical, parallel.
- **Communicating** them to the proposer: +2,900–3,800 prompt tokens (+45–66%), inside a proposal stage that consumed **78–79% of the 240 s compute budget**.

Refinement per review: the 78–79% figure is the proposal-generation share of the *whole* prompt — instructions, catalog, observation block, accumulated history — not all of it attributable to observation serialization. The observation-attributable portion is the incremental prefill/decode cost of the ~3.3k-token obs block plus its contribution to context-length quadratic effects. The correct statement is narrower: **observation collection is ~3% of trajectory compute; the observation-bearing *channel* is a first-order driver of proposal cost.**

## 2. What the campaign implies about that channel

The arm-B result (A1 > A2 > A0, see §1–4.2 of the campaign report) separates two facts:

1. **The channel is read.** A2, shown permuted evidence, chased its phantom high-`|lc|` rows *harder* than A1 chased true ones. The model treats the evidence column as instruction-relevant content, not decoration.
2. **Correspondence determines value.** Same rendering format, same instruction — only the truth of the numbers separated protection from damage.

This is **exploratory evidence supporting correspondence** — an ordering consistent with the mechanism, pending confirmation, not a confirmed result.

It also exposes a property worth naming for Horizon's security model: an agent can be *highly responsive* to information about its own internals while being incapable of independently verifying whether that information is true. The observation channel therefore needs externally established integrity, provenance and correspondence — the proposer must never be the authority on its own evidence.

## 3. The four-century-old version of this problem

This is the same problem the technomancy lineage kept hitting (see `blackmirror1.txt` survey): Dee's scrying tables, Llull's wheels, the Zā'irja's matrices were all attempts to render the state of a hidden system into a symbolic surface a human could reason over. Their failure mode was never the aspiration — it was that the mapping between symbol and system was unconstrained, so *apophenia* filled the gap. (The term is Klaus Conrad's, 1958 — coined in psychiatry for the unmotivated perception of meaningful patterns; Peter Carroll later borrowed it for occult epistemics.)

The modern version of the constraint: **the rendered surface must be verifiably coupled to the measured state**, and the experiment must be able to come back null. Our placebo arm exists precisely to police that coupling — which is what separates the observation layer from a scrying table.

## 4. The serious modern lineage

If the question is "derive general reasoning from finite symbolic primitives rather than corpus mass," the living research programs — modern computational analogues of the *Ars Combinatoria* ambition, not documented descendants — are:

- **DreamCoder** (Ellis et al., 2023, *Growing Generalizable, Interpretable Knowledge with Wake-Sleep Bayesian Program Learning*): grows a library of reusable symbolic abstractions from experience, progressively extending its own DSL. Relevant to whether accumulated validated operations could reduce *future* reasoning cost, not just communication cost.
- **Causal Abstractions of Neural Networks** (Geiger et al., 2021): formal method for mapping neural representations onto interpretable causal variables, tested via *interchange interventions*. Directly relevant to correspondence evaluation *beyond* the linguistic channel — a latent channel's correspondence can be probed by interchanging representations between positions and measuring whether outcomes track the interchange.
- **Inducing Causal Structure for Interpretable Neural Networks** (Geiger et al., 2022): interchange-intervention *training* — teaching representations to exhibit specified causal behavior. The bridge from handcrafted symbolic channels toward learned ones.

Prioritize the Geiger line for the immediate channel work; DreamCoder for longer-horizon architecture.

## 5. Candidate channels — four, in serialization-cost order

The linguistic channel was the right *first* choice: auditable, prompt-hashed, placebo-testable today. The campaign prices it honestly. Ordered by serialization cost:

| # | Channel | Cost | What's lost | What's gained |
|---|---|---|---|---|
| I | Language (current) | ~9.4k tokens/cycle | — | auditability, zero training, group-permutation placebo |
| II | Addressable observation store | ~index + on-demand reads | context completeness | obs bandwidth decoupled from context; localized attention |
| III | Symbolic compression | ~100s of tokens | some nuance | 10–30× cheaper channel; permutation still definable |
| IV | Learned latent | ~0 tokens | human readability; needs interchange-style controls | cycles scale to obs bandwidth |

**Addressable store (new candidate, II).** Rather than serializing the full catalog every cycle, give the model a compact index plus a deterministic read-only interface — `OBS.READ(id)`, `OBS.RANK(field, scope, k)`, `OBS.COMPARE(a, b)`, `OBS.FRESHNESS(id)`, `OBS.HISTORY(id, epoch)`. This tests a distinct hypothesis: whether the model needs the *complete* representation in context or merely *reliable access* to it. Costs to measure: extra inference calls per step, retrieval-selection bias (the model may inspect what's already salient). Not necessarily superior to compression — a different question.

**Correction to v1:** latent channels are not unplaceboable — interchange interventions (shuffled latent assignments between positions, or genuine-vs-swapped conditioning vectors) provide analogous correspondence controls, per the Geiger methodology. The barrier for L-I is training cost and auditability, not the absence of a control design.

## 6. Compression economics — Amdahl applies

Character compression ≠ token compression, and channel compression ≠ system acceleration. On the measured 9.4k-token prompt (~3.3k observation block):

- 10× compression of the *observation block alone*: 9,400 → ~6,385 tokens — **a 32% total reduction, 1.47× prompt compression**, not 10×.
- Realized inference acceleration will differ again (attention caching, decode length, prefill-vs-decode split).

The observation channel is one component of the *cognitive transaction*: instructions, candidate catalog, edit spec schema, accumulated feedback/history, and the model's own output all pay serialization tax. Compressing the channel eventually exposes the rest — the long-run version of this work is compressing the **whole transaction**, not one field.

## 7. The symbolic channel needs three levels plus a pretest

Jumping straight to an arbitrary compressed code risks conflating two failures: *information lost* vs *information preserved but undecodable by this model*. A conventional pretrained model may parse a compact-but-familiar schema far better than an opaque vocabulary. Proposed ladder (each level pairs genuine + permuted):

| Level | Representation | Purpose |
|---|---|---|
| S-I | Compact, semantically familiar schema (`L14.DP lc=-0.03 ar=0.82`) | Remove redundant prose, keep decodability |
| S-II | Fixed symbolic vocabulary (`O:042|L:17|F:MLP|LC:-03|AR:82`) | Test deeper compression and learned association |
| S-III | Compositional symbolic language | Reusable abstractions; program-induction direction |
| L-I | Learned latent | Nonlinguistic communication; deferred pending symbolic results |

**Mandatory pretest — comprehension**: before any self-editing run, verify the model can *decode* the encoding: given a rendered obs row, ask factual probes (which layer? which family? higher or lower contrast than X?). If it can't decode, S1-vs-S2 measures literacy, not correspondence. Pretest is protocol, not optional.

Also measured before encoding selection: **actual tokenized length** of candidate encodings. Punctuation-heavy schemas can tokenize *worse* than short prose under BPE — pick the encoding by measured tokens, not character count.

## 8. Observation identity — locality as a formal data property

Each observation needs an externally-established frame of reference, so that *false* and *stale* are distinguishable states:

```
O = (a, t, v, m, p)
    a  address of observed component
    t  observation time / edit epoch
    v  measured value
    m  measurement method and context
    p  provenance + model-state identity (checkpoint hash)
```

A self-editing system changes the very state its observations describe — `t` and `p` are what let a later trajectory distinguish "this observation was wrong" from "this observation was right about a model that no longer exists." Identity and freshness are established by the observer infrastructure, never asserted by the proposer. This is also the formal hook for the staleness question the campaign left unresolved.

## 9. Formal objective — causally faithful representational compression

The overarching research objective, stated once, precisely:

```
min  C(Z)                                   subject to

     U(Z) ≥ U(X) − ε                        (utility floor)
     U(Z_true) − U(Z_shuffled) > δ          (correspondence margin)
     capability & safety constraints         (enforced independently)
```

`X` = measured internal state, `Z` = compressed representation, `A` = proposed intervention, `Y` = evaluated outcome, `C` = measured computational cost, `U` = intervention utility. Tolerances ε, δ set at preregistration. The objective is channel-agnostic — it holds whether Z is text, symbols, a DSL, or a learned latent.

## 10. Compressing observations vs compressing understanding

There is a distinction worth marking early:

- **Compressing an observation** → `L14.DP.LC=-0.03`. Reduces communication overhead.
- **Compressing an understanding** → a reusable, empirically validated conditional rule: `UNDER CONDITIONS X: OPERATION Y PRODUCES EXPECTED EFFECT Z WITH UNCERTAINTY U`. Reduces *future reasoning* overhead.

The second is the DreamCoder direction — accumulating validated operational abstractions rather than re-deriving the same relationships through expensive inference each cycle. A computational reading of the grimoire concept: not a repository of knowledge, but a library of verified, composable operations — subject to revalidation whenever the underlying model state changes (see §8, provenance).

## 11. Bottom line

The exploratory campaign provides *preliminary empirical support* for FS-05: the representation of internal state measurably influences behavior, and that influence depends on the representation's correspondence to the state. What the campaign also did is price the current channel — most of the compute budget spent *saying* the evidence rather than using it.

The original JARVIS objective was removing the language barrier between human intention and machine operation. Gate 1.2 has surfaced a second, structurally identical barrier: **the language-bandwidth barrier between a machine's reasoning apparatus and its own internal state.** The open question both directions share:

> What is the smallest faithful representation necessary for intelligence to act effectively upon what it knows?

That question stands independently of whether this particular self-editing architecture succeeds — and it deserves its own research track (HZN-OC-01 proposal drafted separately).
