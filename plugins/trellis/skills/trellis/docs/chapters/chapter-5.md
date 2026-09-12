# Chapter 5: Personas Are Load-Bearing

A persona is a design constraint, not a description. A seven-dimension portrait of a specific person in a specific situation anchors every outcome to a real use case. Without it, outcomes float free — plausible but untethered from reality.

## Key Principles

- **A persona is a design constraint, not a description.** "Sarah, 34, marketing manager" is a description. It tells you nothing useful. A design constraint tells you: Sarah is on her phone between meetings, has three minutes, needs to confirm an event booking, and will abandon the process if it requires more than two screens. That changes what you build.

- **Seven dimensions make a persona load-bearing.** Name and role. Situation and context. Goal and urgency. Technical comfort. Constraints (time, device, connectivity). Prior experience with similar systems. What failure looks like for this person. Each dimension eliminates a class of assumptions.

- **Every outcome belongs to a persona.** An outcome written for "users" is an outcome written for no one. When you specify that Sarah — on her phone, in a hurry, moderately technical — needs to complete a booking, every design decision is constrained by her situation. The walkthrough must work for her specifically.

- **Different personas produce different outcomes.** The same capability — "book an event" — produces different outcomes for different personas. A first-time visitor needs more guidance. A returning customer needs fewer steps. An administrator needs bulk actions. One feature, multiple personas, multiple outcomes.

## Red Flags

- Personas described as demographics. Age, gender, job title, and location are not design constraints. They tell you nothing about how the person will interact with your system. Add situation, urgency, constraints, and failure conditions.

- Outcomes written for "users" rather than a specific persona. Every time you see "the user" in an outcome, replace it with a named persona. If you cannot decide which persona this outcome serves, the outcome may be trying to serve everyone and serving no one.

- All outcomes sharing one persona. If every outcome is written for the same persona, you are missing perspectives. Who else uses this system? What about the person who uses it rarely? The person who uses it under pressure? The person who makes mistakes?

## What This Means for You

Review your personas. For each one, check: do you know their situation, their constraints, their urgency, and what failure looks like for them? If your persona reads like a LinkedIn profile, it is not yet a design constraint.

Then check your outcomes. Is every outcome anchored to a specific persona? When you read the walkthrough, can you picture that specific person in that specific situation following those steps?

Next: Chapter 6 introduces contracts — the data that flows between outcomes — and why mapping them prevents the most common build failures.
