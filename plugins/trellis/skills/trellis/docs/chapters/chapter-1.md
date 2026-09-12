# Chapter 1: It's All About Clarity

The fundamental problem of AI-assisted development is specification precision, not technical skill. The AI can build from a clear specification. It cannot invent domain knowledge.

This is the single most important idea in the entire methodology. You are not learning to code. You are learning to specify — to describe what people need to do, what should happen when they do it, and how you will know it worked. The AI handles everything else.

## Key Principles

- **Specification precision is the bottleneck, not technical ability.** The AI can generate code, configure databases, wire up APIs. What it cannot do is know that your customers need to see their booking confirmation before they leave the page.

- **Domain knowledge is yours alone.** You know your users, your workflows, your edge cases. No model, however capable, has sat in your meetings, handled your customer complaints, or watched someone struggle with your competitor's product.

- **Clarity is measurable.** A clear specification can be read by someone unfamiliar with the project and understood without follow-up questions. If it requires a conversation to interpret, it is not yet a specification.

- **The AI will fill gaps with guesses.** When a specification is vague, the AI does not stop and ask. It makes a plausible assumption and builds from it. The result looks correct until a real user encounters the assumption.

## Red Flags

- Vague requirements treated as specifications. "Users should be able to manage their profile" is not a specification. It is a wish. A specification describes what managing a profile looks like step by step.

- Assuming the AI will figure out domain details. If you do not state that a cancelled booking should trigger a refund within 48 hours, it will not happen. The AI does not know your refund policy.

- Confusing technical fluency with specification quality. You do not need to know how a database works. You need to know that when a teacher marks an assignment, the student should see the grade within five seconds.

## What This Means for You

Your job is not to learn the technology. Your job is to be ruthlessly specific about what people need to do in your system and what should happen when they do it. Every chapter that follows builds on this foundation.

If you have already started writing outcomes for your project, review them now. Can someone who knows nothing about your domain read each outcome and understand exactly what should happen? If not, the specification needs work — and that work is yours to do.

Next: Chapter 2 explains how to collaborate on the technical decisions that sit between your specification and the built system.
