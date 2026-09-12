# Chapter 4: Outcomes Should Be Specific

The six-field outcome format — persona, trigger, walkthrough, verification, contracts exposed, contracts consumed — eliminates the gaps that cause build failures. Every field is required. A missing field is an ambiguity that will become a bug.

## Key Principles

- **An outcome is the unit of specification.** Not a user story. Not a feature. An outcome: a six-field structure that names who does what, when, how, and how you confirm it worked. This is what Trellis builds from.

- **Automated tests confirm the code does what it was told. Verification confirms it was told to do the right thing.** Tests check that a function returns the correct value. Verification checks that the function should have been written in the first place — that the behaviour is what the persona actually needs.

- **Every field carries weight.** The persona anchors the outcome to a real use case. The trigger defines when it starts. The walkthrough specifies the behaviour. The verification tells you how to check it. The contracts map dependencies. Skip a field and you create a gap the AI will fill with a guess.

## The Four Quality Traps

- **Vagueness.** "The user sees their dashboard" — what is on the dashboard? What numbers? What state? Specify the content.

- **Technical language.** "The API returns a 201 with the event payload" — the domain expert cannot verify this. Rewrite in terms of what the persona sees and does.

- **Happy path only.** The outcome describes what happens when everything goes right. What happens when the event is sold out? When the payment fails? When the network drops? Failure paths are outcomes too.

- **Kitchen sink.** An outcome that tries to cover too much — booking, payment, confirmation, and review in one walkthrough. Split it. Each outcome should describe one coherent sequence.

## Red Flags

- An outcome with no verification field. If you cannot describe how to check that it worked, you do not yet know what "worked" means.

- A walkthrough written in technical terms. If the walkthrough mentions databases, endpoints, or status codes, it is not written for the domain expert. Rewrite it in terms of what the persona experiences.

- An outcome with no failure path. Every outcome that can fail should have a companion outcome describing what happens when it does.

## What This Means for You

Take one outcome from your project and run it through the six fields. Is the persona specific? Is the trigger clear? Can you follow the walkthrough as the persona and know at every step what you should see? Can you verify the result without asking a developer? Do you know what data it produces and what data it needs?

If any field is missing or vague, fill it in now. That gap is exactly where the build would go wrong.

Next: Chapter 5 explains why personas are not demographics — they are design constraints.
