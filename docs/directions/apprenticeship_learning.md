# Apprenticeship Learning

> Status: long-term direction.
> Purpose: define a durable Alan capability direction without implying current behavior,
> architecture contract, or near-term implementation work.

## Summary

Alan should eventually be able to learn from repeated interaction, examples, corrections,
and successful task histories in a way that reduces future supervision.

This is not just "more memory." The long-term goal is for Alan to extract reusable
behavioral patterns from interaction history and promote them into more capable default
behavior over time.

The working metaphor is apprenticeship:

1. A human demonstrates or corrects.
2. Alan observes repeated patterns.
3. Alan internalizes stable ways of working.
4. Later tasks require less explicit instruction and less human intervention.

## Why This Matters

A general agent does not become practically useful only by having more tools or a larger
context window. It becomes more useful when it can:

1. remember what quality looks like,
2. infer what the operator consistently wants,
3. reuse successful workflows,
4. avoid repeating previously corrected mistakes,
5. operate with less micromanagement over time.

This direction is therefore about capability growth through interaction, not just storage.

## Core Claim

The long-term value of Alan is not only that it can execute a turn, but that it can be
trained through usage.

Over many turns and sessions, Alan should be able to accumulate something closer to
practical know-how:

1. preferences,
2. judgment criteria,
3. workflow patterns,
4. escalation boundaries,
5. benchmark-solving strategies,
6. domain-specific ways of working.

## Working Definition Of The Agent

This direction assumes a specific framing:

1. Alan, at the workspace level, is the persistent agent.
2. Sessions, turns, and subagents are execution forms of that agent.
3. Subagents are not necessarily separate enduring selves; they may be task-local modes,
   workers, or projections of the same underlying agent identity.

This framing matters because learning should attach primarily to the persistent agent,
not to short-lived execution fragments.

## What Should Be Learned

Examples of learnable artifacts may include:

1. operator preferences,
2. recurring task decomposition patterns,
3. effective tool-use sequences,
4. correction-derived "do not repeat" rules,
5. quality rubrics for acceptance or escalation,
6. benchmark-specific solution heuristics,
7. reusable intermediate abstractions that later become skills or policies.

The goal is not to memorize transcripts. The goal is to promote stable patterns.

## Tentative Learning Pipeline

This direction suggests a future pipeline like:

1. Observe:
   collect candidate signals from interaction history, memory, outcomes, and corrections.
2. Extract:
   identify recurring patterns rather than isolated examples.
3. Evaluate:
   decide whether a pattern is reliable enough to reuse.
4. Promote:
   store the pattern in a durable, structured form.
5. Apply:
   use it in future planning, routing, tool use, or evaluation.
6. Revalidate:
   confirm that promoted patterns continue to help rather than drift.

This should be treated as capability promotion, not unbounded online self-modification.

## Root Agent vs Execution Agent

One open design axis is where learning authority should live.

Possible split:

1. execution agents / subagents:
   detect candidate lessons while doing local work.
2. root agent:
   decides which lessons are durable enough to adopt at workspace scope.

This suggests a bias toward centralized promotion:

1. local workers discover,
2. the persistent agent judges,
3. promoted knowledge becomes shared future capability.

That structure would reduce fragmentation and make learning more auditable.

## Relationship To Memory

Memory is an input to this direction, but not the same thing.

Memory answers:

1. what happened before,
2. what matters now,
3. what context should be carried forward.

Apprenticeship learning asks a harder question:

1. what stable pattern should change how Alan behaves next time.

In other words:

1. memory preserves experience,
2. apprenticeship turns experience into behavior.

## Relationship To Skills, Policies, And Prompts

If this direction succeeds, some learned patterns may eventually become:

1. skill refinements,
2. routing heuristics,
3. evaluation rubrics,
4. governance hints,
5. reusable benchmark strategies,
6. stronger defaults for specific operators or workspaces.

The important constraint is that promotion should stay explicit, inspectable, and
testable. Alan should not rely on opaque drift as its main learning mechanism.

## Non-Goals

This direction does not imply:

1. uncontrolled self-editing of prompts or code,
2. blind trust in raw conversation history,
3. turning every subagent into an independent long-term persona,
4. replacing harness evaluation with online improvisation,
5. treating all user feedback as equally durable learning signal.

## Open Questions

Major unresolved questions include:

1. What is the right representation for a learned pattern?
2. How is promotion decided and by whom?
3. How should conflicting lessons be resolved?
4. What should remain operator-specific versus globally reusable?
5. How much autonomy should learning have before human review is required?
6. How should learned behavior be benchmarked and rolled back?

## Relationship To Benchmarks

A long-term motivation for this direction is that apprenticeship may let Alan improve
through guided use before tackling harder public agent benchmarks.

The expected sequence is:

1. learn from real operator interaction,
2. reduce supervision on repeated internal tasks,
3. validate behavior with harness and internal benchmarks,
4. gradually apply the resulting capabilities to public agent and AI benchmarks.

The benchmark goal should remain downstream of genuine capability growth, not the sole
reason to build the system.

## Promotion Criteria

This direction should move closer to architecture or implementation only when:

1. Alan has a clearer representation for durable learned behavior.
2. The root-agent versus subagent learning boundary is better defined.
3. Harness-based evaluation can measure whether learning truly reduces intervention.
4. The design can preserve auditability, reversibility, and small-kernel discipline.
