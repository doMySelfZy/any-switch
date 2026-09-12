# Chapter 7: Design the Outcome Twice

A first draft is thinking. A reviewed draft is a specification. Write it once to get the idea down. Review it against the four quality traps to catch what was missed. This two-pass approach is the difference between outcomes that feel right and outcomes that build correctly.

The first time you write an outcome, you are discovering what you mean. That is valuable work — but it is not yet a specification. It becomes a specification when you review it with fresh eyes and catch the assumptions you made without noticing.

## Key Principles

- **A first draft is thinking. A reviewed draft is a specification.** The first pass captures your intent. The second pass tests it against reality. Both are necessary. Skipping the review is the single most common cause of build failures that look like "the AI built the wrong thing."

- **The four quality traps are your review checklist.** On the second pass, check each outcome for vagueness, technical language, happy-path-only thinking, and kitchen-sink scope. These four traps catch the majority of specification problems before they become build problems.

- **Review as the persona, not as yourself.** When you review the walkthrough, step into the persona's situation. Are you on a phone with three minutes? Are you a first-time user who has never seen this interface? Does the walkthrough still make sense from their perspective?

## The Four Quality Traps (Review Checklist)

- **Vagueness.** Does every step describe something specific and observable? "The user sees relevant information" is vague. "Sarah sees her upcoming bookings sorted by date, with the event name, date, and ticket count for each" is specific.

- **Technical language.** Can the persona follow every step without technical knowledge? If a step mentions APIs, databases, or status codes, rewrite it in terms of what the persona sees and does.

- **Happy path only.** What happens when something goes wrong? If the outcome only describes the success case, add failure paths. What if the event is full? What if the payment is declined? Each significant failure is its own outcome.

- **Kitchen sink.** Does the outcome try to do too much? If the walkthrough covers more than one coherent action, split it into separate outcomes connected by contracts.

## Red Flags

- Outcomes that have never been reviewed. If you wrote it once and moved on, it is a first draft. Schedule the second pass.

- Reviewing your own outcomes immediately after writing them. You will read what you meant, not what you wrote. Review them the next day, or ask someone else to read them.

## What This Means for You

Pick three outcomes from your project. Read each one as if you are the persona, in their situation, with their constraints. Run each through the four quality traps. You will find gaps. Fill them. That is the work.

Next: Chapter 8 shows how the contract map determines your build order — phases derived from structure, not guesswork.
