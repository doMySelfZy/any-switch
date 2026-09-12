# Chapter 15: Good Interfaces Are Specified, Not Designed

Interface quality is a specification problem. The walkthrough already describes behaviour — the missing piece is being explicit about visible state, feedback, and what the persona should understand at each moment. `*ui` loads five principles and component defaults.

You do not need to be a designer. You need to be specific about what the persona should see and do at every step. "Sarah sees her bookings" is vague. "Sarah sees upcoming bookings sorted by date, each showing event name, date, and ticket count, with a cancel button" is a specification.

## Key Principles

- **Every interface element that communicates status, confirms an action, or guides the next step is a specification problem, not a design problem.** You do not need to choose colours or fonts. You need to specify what information is visible, what feedback the persona receives, and what they should do next.

## The Five Principles

- **Visible state.** The persona should always be able to tell what state the system is in. Is the booking confirmed? Is the payment processing? Is the form saved? If the state is not visible, the persona is guessing.

- **Single focus.** Each screen should ask the persona to do one thing. If a screen asks for payment details while also showing event recommendations, the persona's attention is split. One action per screen.

- **No dead ends.** Every screen should offer a clear next step. If the persona completes an action and sees no indication of what to do next, the interface has a dead end.

- **Clear confirmation.** When the persona completes an action, they should see explicit confirmation. Not a subtle change — a clear message. "Your booking is confirmed" with the relevant details. Ambiguous feedback creates anxiety and repeat actions.

- **Recoverable errors.** When something goes wrong, the persona should see what went wrong and how to fix it. "Something went wrong" is not recoverable. "Your payment was declined — please check your card details or try a different card" is recoverable.

## Red Flags

- Verification steps that can only be followed visually. If a verification step says "check that the page looks correct," it is not a verification step. Specify what should be visible: which elements, which data, which state.

- No keyboard-only step in the checklist. Some personas use keyboards, screen readers, or assistive technology. If your verification checklist does not include a step for keyboard navigation, you are excluding those personas.

- Feedback described as "appropriate" or "clear." These are subjective. Specify the exact message, the exact information displayed, the exact next step offered.

## What This Means for You

Review your outcome walkthroughs. At each step, ask: what does the persona see? What do they understand about the system's state? What do they do next? If any of these are ambiguous, make them explicit. That is interface specification.

Next: Chapter 16 explains how ODD handles change — and why it is cheaper than you expect.
