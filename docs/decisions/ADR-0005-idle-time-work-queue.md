# ADR-0005: Bounded attention lifecycle and idle-time work queue

**Status:** proposed

## Decision

JARVIS has one owner-controlled identity and one permission authority across
foreground interaction and bounded background work. "Sleep" and "wake" describe
engineered attention modes, not subjective experience or separate agents:

- **Idle:** a low-cost, mostly deterministic sentinel checks authorized events
  and timers; no LLM runs just to remain "aware."
- **Foreground:** user input receives priority and may use local inference.
- **Background:** a permitted, budgeted job runs only when foreground work is
  absent; it yields, checkpoints, or cancels when user input arrives.

The sentinel records typed events and unresolved concerns in a persistent,
inspectable journal. An event records source, observed time, evidence reference,
deduplication key, deadline/next check, and disposition (attend, defer, ignore).
A concern separates observation from inference, with confidence/uncertainty,
importance, applicable permission scope, next check, and resolution evidence.
Model-suggested importance or a self-selected check time is a proposal bounded
by owner-configured limits; neither creates a wake source or grants authority.
Clock and resource-adaptive checks use ordinary timers, not continuous inference.

Proactivity is expressed as typed, queued goals with declared completion
criteria, not an always-thinking model loop. "Satisfied" means a criterion was
verified or a bounded explanation was recorded for why it could not be met.
Jobs have limits on time, tokens, retries, and jobs per tick, and cannot create
unbounded child jobs. The owner can pause or stop background work at any time.

Background outputs are diagnostics or proposals, such as review candidates and
approved-Knowledge context suggestions. They never silently update Knowledge,
purpose, permissions, skills, or code; execute tools; or speak uninvited.
Any later side effect requires its own scoped, revocable authorization.

The shared identity draws only on a valid owner-approved `Purpose.md`, approved
Knowledge, and explicitly labelled unverified history. A missing or pending
purpose note is not synthesized or treated as permission. Personal identity,
concerns, and the queue stay in the user's ignored local vault, not the public
source repository. The journal should contain reason codes and evidence
references rather than raw transcripts, audio, or prompts by default.

## Context

- The approved-purpose reader and review-gated memory exist, but no durable
  attention journal, scheduler, background inference owner, or job checkpoint
  exists today. The UI system monitor displays metrics; it does not raise
  internal concerns.
- `llama-completion.exe` starts per request. Current runtime logs record total
  inference time, not reliable separate model-load, prompt-evaluation, and
  generation times. A synthetic long-context benchmark showed prompt
  evaluation can dominate latency; model residency is not yet a proven fix.
- Voice deactivation can discard a late reply but does not cancel its running
  inference. Foreground priority requires actual cancellation/defer behavior
  before background jobs use the heavyweight model.
- The desktop CPU build should serialize 7B-class inference rather than run
  competing foreground and background model instances.

## Wake and authority boundaries

- Initial event sources are explicit user input and owner-enabled local timers
  or read-only checks. No always-listening microphone, unapproved OS watcher,
  network source, or external integration is implied by this ADR.
- A wake decision may investigate a concern; it cannot authorize a tool,
  change the constitution, approve memory, or convert an inference into fact.
  Sources and consequences are shown to the owner; "ignore" and "defer" are
  valid outcomes. Quiet hours and resource ceilings are owner-controlled.
- One inference owner arbitrates the lanes. Foreground requests prevent new
  background inference and cancel or defer any in-flight background work.
  Jobs checkpoint only verified progress; cancellation and restart cannot
  claim success or duplicate consequential actions.
- The journal and queue live in an inspectable `Jarvis/` vault subtree,
  separate from `Knowledge/` and `Review/`. Files need bounded size, atomic
  updates, deduplication, and recovery rules; editing a journal entry cannot
  grant new permissions. The precise storage format remains to be chosen.

## Adoption sequence and checks

1. Measure inference phases with synthetic prompts and reduce structured-intent
   context while preserving short follow-ups. Do not benchmark against a live
   conversation or private vault content.
2. Add a deterministic sentinel and durable, read-only event/concern journal
   without background LLM work. Test timer delivery, missed ticks, duplicate
   events, restart recovery, owner pause, and truthful outcome labels.
3. Implement one inference owner with bounded cancellation, foreground
   priority, and resumable checkpoints. Test interruption during inference,
   crash recovery, and no false completion or duplicate action. Only then
   enable background model jobs.
4. Add a small set of budgeted, opt-in reflection/validation jobs. Their
   results are proposals or diagnostics pending owner review, never automatic
   changes to purpose, Knowledge, tools, or permissions.
5. Benchmark a single resident local model against the subprocess path after
   context reduction. Adopt residency only if measured latency, RAM, shutdown,
   cancellation, and offline-boundary behavior justify it. A separate process,
   smaller model, circadian adaptation, and sandboxed simulation are later
   design decisions, not first-milestone features.

## Alternatives rejected

- An always-running "think loop" that lets the model decide when it is done:
  unbounded, unauditable, and violates the intent-to-fulfillment contract.
- Treating background and foreground as separate identities or authorities:
  continuity must not depend on two self-governing personalities.
- Letting background work write to Knowledge or invoke tools directly:
  initiative would silently become authority.
- A second concurrent inference on the same CPU build: contention makes
  voice turns slower and less reliable, not faster.
- An unbounded self-chosen wake cadence or "dream" mode in the first release:
  neither is needed for reliable, governed attention.

## Consequences

- New modules will own typed events, attention policy, journal/queue, and
  inference arbitration. A UI thread may host an initial sentinel, but the
  design must permit a later separate process without a second identity.
- Resident inference remains optional. A local-only process or loopback server
  needs an explicit ADR-0001 offline-boundary decision, access restrictions,
  resource measurements, and reliable shutdown before adoption.
- This ADR records a proposed direction only. It does not claim that sleep,
  wake scheduling, reflection, model residency, or proactive actions exist.
