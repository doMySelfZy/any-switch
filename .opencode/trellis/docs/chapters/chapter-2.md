# Chapter 2: The Right Division of Labour

Technical decisions have domain consequences. Collaborate on them — understand the options and tradeoffs, decide with understanding, record the reason in CLAUDE.md. Don't delegate blindly and don't try to make them alone.

You are not a developer. But you are the person who will live with the consequences of every technical choice. The right division of labour is not "you do the domain, the AI does the tech." It is: the AI presents options with tradeoffs explained in domain terms, and you decide.

## Key Principles

- **Technical decisions have domain consequences.** Choosing a database is not a technical detail — it determines how fast your users see their data, how much you pay per month, and what happens when a thousand people log in at once. You need to understand the consequences, not the implementation.

- **Collaboration on technical decisions produces better outcomes than delegation.** When you say "just pick whatever works," you lose the ability to understand why something was chosen. When it causes a problem later, you cannot reason about it. Ask for options. Ask what changes for your users under each option. Then decide.

- **Record decisions in CLAUDE.md.** Every technical decision should be written down with the reason it was made. Not the technical justification — the domain reason. "We chose X because our users need Y and X supports that better than Z." Future sessions read CLAUDE.md. If the reason is not there, the decision may be silently reversed.

## Red Flags

- Accepting a technical choice without understanding why. If the AI recommends something and you cannot explain the reason to a colleague in plain language, you do not yet understand the decision. Ask again.

- Making technical decisions without asking for options. You might have strong instincts about what your system should do. But instinct without options is guessing. Ask the AI to present two or three approaches with their tradeoffs described in terms of what your users will experience.

- Decisions made but not recorded. A decision that exists only in the conversation is a decision that will be forgotten. Trellis reads CLAUDE.md at the start of every session. If a decision is not there, it does not exist.

## What This Means for You

When a technical question comes up during a build, do not wave it through and do not try to answer it from first principles. Ask the AI to explain the options in terms you understand — what changes for your users, what costs more, what limits you later. Decide. Record the decision and the reason.

Next: Chapter 3 draws the line between features and outcomes — the difference between a category and a specification.
