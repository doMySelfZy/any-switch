# Trellis — Knowledge Base

This is the complete ODD methodology reference. Read it when you need to understand why the method works, not just how to use it. The practices in Trellis are not arbitrary — each one exists because a specific kind of failure was common and costly.

---

## 1. Why the 60-Year Assumption Collapsed

For sixty years, software development operated on a single assumption: to build software, you needed to know how to code.

This was not a choice. It was a constraint. The only people who could translate a requirement into working software were people who had spent years learning the languages, frameworks, and patterns of software engineering. Domain experts — teachers who understood pedagogy, clinicians who understood patient care, lawyers who understood contract law — could describe what they needed, but they could not build it. They needed an intermediary: a developer who would translate their domain knowledge into code.

This translation process was the source of most software project failures. The developer did not understand the domain deeply enough to ask the right questions. The domain expert did not understand software deeply enough to describe their needs precisely. The resulting system was always a partial translation — correct in structure, wrong in substance.

AI-assisted development has collapsed this assumption. A domain expert who can describe precisely what they need — using structured, unambiguous specifications — can now direct an AI coding assistant to build it. The translation step still exists, but the domain expert is doing the translating, not delegating it.

This is not the same as saying anyone can build anything. It means that domain expertise, combined with a structured methodology for describing systems, is now sufficient to build serious software. The methodology is ODD.

---

## 2. Writing Code vs Building Systems

There is a distinction that most guides to AI-assisted development miss. They teach you how to write code with AI. ODD teaches you how to build systems.

Writing code means producing correct syntax. It means a function that takes an input and returns an output. It means a component that renders correctly. Writing code is a craft skill. AI is extraordinarily good at it.

Building systems means producing software where the parts connect correctly, where the data flows as intended, where each capability depends on the ones before it, where the whole is coherent. Building systems is a design skill. AI is only as good at it as the design you give it.

The failure mode of AI-assisted development without a methodology is this: you describe what you want, the AI writes it, and you get something that looks like software. It has screens and buttons and a database. But it does not hold together. The data produced in one part does not reach the part that needs it. The rules you care about are not enforced. The people who should be able to do things cannot. The system works, technically. But it does not work as a system.

ODD's job is to give you the design skill before you approach the AI.

---

## 3. Why Agile Fails Non-Technical Builders

Agile methodology was designed for professional software teams. It assumes:

- Multiple full-time developers with deep technical knowledge
- A product owner who can make technical trade-off decisions rapidly
- Sprint ceremonies that presuppose software engineering vocabulary
- Continuous integration and deployment infrastructure
- A shared codebase that a team maintains and evolves together

Non-technical domain experts building with AI have none of these. They work alone or in very small groups. They are building something that does not yet exist. They cannot make technical trade-off decisions because they do not know what the trade-offs are. And the "team" is an AI that has no memory between sessions and no stake in the coherence of the whole.

When non-technical builders try to apply agile to AI-assisted development, they discover that the ceremonies produce no shared understanding (because there is no shared technical context), the sprints produce disconnected pieces (because there is no architectural continuity), and the retrospectives identify the same problems every cycle (because the root cause — an absence of explicit design — is never addressed).

ODD is not anti-agile. It is a pre-agile methodology. It produces the specification that agile teams would normally grow incrementally. But it produces it upfront, in a form that an AI can build from without ambiguity.

---

## 4. Why User Stories Leave the Hard Part Unspecified

User stories have the form: "As a [role], I want to [action] so that [goal]."

This is useful for describing intent. It is useless for describing a system.

Consider: "As a teacher, I want to approve student applications so that I can manage my cohort."

This says nothing about:
- What information the teacher needs to see to make an approval decision
- What happens to the application when it is approved (which other parts of the system are affected?)
- What the student sees or receives as a result
- Whether approval is reversible
- What happens if the teacher approves when the cohort is already full
- What data is produced by the approval that other outcomes depend on

These omissions are not oversights. They are structural. The user story format is designed to express desire, not to specify behaviour. Developers fill in the gaps. Domain experts never see the decisions being made. The resulting software reflects the developer's assumptions, not the domain expert's knowledge.

ODD replaces user stories with outcomes. Outcomes specify every detail that a user story leaves out.

---

## 5. What ODD Is and Why It Works

Outcome-Driven Development is a methodology for designing software from the user's experience backward to the technical implementation.

It starts with a concrete question: what does each type of person in your system actually experience? Not what they want in the abstract, but what they see, what they do, and what changes as a result of what they do.

From those experiences, it derives everything else: the data that must exist, the logic that must run, the interfaces that must connect. Architecture is not designed separately and then populated with features. Architecture emerges from the aggregation of precisely-described outcomes.

ODD works for non-technical builders for three reasons:

**First, it works in domain language.** You do not need to know about APIs or database schemas or React components to write an outcome. You write what a teacher does when they approve a student. You write what the student sees when they receive confirmation. The technical translation is handled by AI.

**Second, it makes the implicit explicit.** The hard work in software design is not writing code — it is deciding what the code should do. ODD forces those decisions to be made before building begins. A well-written set of outcomes contains every decision that matters. Nothing is left to the AI's interpretation.

**Third, it creates a specification that AI can build from.** Claude Code, given a well-written outcome with all 6 fields complete, can build the implementation. Given a user story with 3 fields, it has to guess. ODD closes the specification gap.

---

## 6. The Outcome Template — All 6 Fields

Every outcome in an ODD plan has exactly 6 fields. All 6 are required. A missing field is not a shortcut — it is an ambiguity that will manifest as a bug or a rebuild.

---

**OUTCOME NAME**

A short, active-voice phrase that describes what happens from the system's point of view. Not "Teacher Dashboard" (that is a screen, not an outcome). Not "Manage Applications" (that is a category, not an outcome). Good names: "Teacher approves student application", "Student receives enrolment confirmation", "Admin generates monthly attendance report".

The name should make clear: who does it, what they do, and (implicitly) what changes as a result.

---

**PERSONA**

Which of your defined personas does this outcome belong to? Write the persona name and, briefly, their defining characteristic: "Maya, the cohort teacher — permanent staff who manages day-to-day student relationships".

One outcome = one primary persona. If multiple personas are involved, you probably have multiple outcomes that need to be separated.

---

**TRIGGER**

What causes this outcome to begin? The trigger is the moment the persona enters the experience described in this outcome.

Triggers are either:
- An action by the persona: "Maya opens the Applications tab in her dashboard"
- An event in the system: "A student submits their application form" (which triggers an outcome for Maya as well as one for the student)
- A scheduled event: "At 9am on Monday, the weekly summary email is generated"

The trigger is the entry point. Without it, the outcome has no context.

---

**EXPERIENCE**

The walkthrough. This is the longest and most important field. It describes, step by step, what the persona sees and does from trigger to completion.

Write it as a numbered list:
1. Maya sees the Applications tab showing 3 new applications since her last visit.
2. She clicks the first application — Jordan Chen's. A detail panel opens to the right.
3. She reads Jordan's personal statement, previous qualifications, and teacher reference.
4. She clicks Approve. A confirmation dialog asks: "Approve Jordan Chen for [Course Name]?" with an optional notes field.
5. She adds a note: "Strong application, particularly the project work experience."
6. She clicks Confirm Approval.
7. The application moves from the New Applications list to the Approved list.
8. A success notification appears: "Jordan Chen's application approved."
9. Jordan receives an email within 60 seconds: "Congratulations — your application has been approved."

The experience must be complete enough that someone who had never seen the system could use it as a test script. If a step is ambiguous, it will be built wrong.

---

**CONTRACTS CONSUMED**

What data does this outcome need that it did not itself create? Contracts consumed are the inputs.

List them by name and describe what is needed:
- `student-application`: The submitted application data — name, contact details, personal statement, qualifications, teacher reference.
- `course-configuration`: The course details — name, capacity, start date, requirements.
- `teacher-session`: The authenticated teacher's identity and permissions.

Contracts consumed connect this outcome to the outcomes that produce its inputs. If an outcome tries to consume a contract that no other outcome produces, there is a gap in the plan.

---

**CONTRACTS EXPOSED**

What data does this outcome produce that other outcomes will need? Contracts exposed are the outputs.

List them by name and describe what is produced:
- `application-decision`: The approval record — application ID, teacher ID, decision (approved/rejected), notes, timestamp.
- `student-notification-trigger`: Event that triggers the student notification system — student email, course name, decision.

Contracts exposed are the connective tissue of the system. An outcome that exposes nothing is a dead end. Most outcomes should expose something.

---

## 7. Writing Good Outcomes — The 4 Traps

Most outcome-writing mistakes fall into one of four categories. Knowing them prevents most rework.

---

**Trap 1: Vagueness**

Could this outcome be implemented in more than one way that satisfies its wording but produces different user experiences? "The customer receives a confirmation after booking." Confirmation could be a screen, an email, a text message, or all three. The wording does not distinguish between them.

**Fix:** Add specificity. "The customer sees a confirmation screen immediately after payment, displaying the booking reference, event name, and date. A confirmation email with the same details and a calendar invitation arrives within five minutes."

---

**Trap 2: Technical Language**

Does any part of this outcome describe implementation rather than behaviour? "A POST request is sent to the booking endpoint, and a 201 response confirms the reservation."

The AI will build exactly that. But the outcome says nothing about what the person sees, how they know it worked, or what happens if it fails.

**Fix:** Rewrite every technical phrase in domain language. "The booking is confirmed immediately on screen, and the customer does not need to refresh or navigate away to see the result."

---

**Trap 3: Happy Path Only**

Does the walkthrough describe only what happens when everything goes right? "The customer selects tickets, pays, and receives confirmation." What happens if the requested number exceeds availability? What happens if payment fails? What happens if their session expires mid-checkout?

A system that only handles happy paths is not a finished system. It is a prototype.

**Fix:** For every action in the walkthrough, ask: what can go wrong? Add the failure cases. "If the requested number exceeds availability, they are informed immediately and shown the current limit. They can adjust their selection."

---

**Trap 4: Kitchen Sink**

Does this outcome try to do more than one thing? Could its walkthrough be split into two or more distinct walkthroughs with separate triggers and separate verification checklists?

"The customer browses events, selects one, books tickets, and manages their booking." That is at least two outcomes: browsing and booking (trigger: curiosity) versus managing an existing booking (trigger: they need to change something). Different triggers. Different verification. Different failure paths.

**Fix:** Split the outcome. Each distinct trigger becomes a separate outcome. Each piece of behaviour with its own verification becomes its own specification.

---

## 8. The 7-Dimension Persona

A persona in ODD is not a user type. It is a precise description of a person's relationship with your system: what they know, what they can do, what they cannot see, and what motivates them.

ODD uses a 7-dimension persona model. Each dimension is a constraint on the outcomes that belong to this persona.

---

**1. Identity**

Who they are in the context of this system. Not their age or job title in the abstract, but their relationship to the system. Specific enough to be real. "Maya Thompson, Cohort Teacher at Westfield Sixth Form College, permanent staff, 8 years in post — a returning customer who has attended at least two previous events and has an active account."

The specificity matters because it prevents scope creep. The outcomes for "Maya" are outcomes for a permanent, experienced teacher with full cohort permissions — not outcomes for a supply teacher, not outcomes for a trainee, not outcomes for a teacher at a different organisation.

---

**2. Reality**

The physical environment they are in when they use this outcome. Device, connection quality, noise level, time pressure. Not their current process — their actual situation at the moment they interact with the system.

"Maya uses the system during form period (20 minutes), on a school laptop with Chrome, stable wifi. Sometimes she checks it on her personal iPhone in the evening at home, where she has more time but is less focused."

Reality shapes UI decisions: mobile-first for evening use, responsive design for different devices, fast interactions for time-pressured sessions.

---

**3. Psychology**

Technical confidence, stress level, tolerance for confusion. Not just whether they can use software — how they feel when using it, how quickly they give up when something is unclear.

"Maya is confident with online systems, low stress when using the platform during a free period, expects the process to take under two minutes. Will abandon if it takes longer or requires more than two steps she does not understand."

Psychology shapes how we write outcomes — specifically the walkthrough and the failure paths. A confident persona tolerates a compact flow. An anxious one needs reassurance at every step.

---

**4. Trigger**

What brought them to this outcome. Not a UI click, but a real-world event. The moment that makes them reach for the system.

"Maya saw an event announcement on the bookshop's social media and wants to reserve a place before it fills up."

Triggers are the opening of every outcome we write. Different triggers produce different emotional and practical starting states — and the platform must meet the persona in that state.

---

**5. History**

What they have done before arriving at this outcome. Their prior experience with the platform — what they already know, what they have already set up, what does not need explaining.

"Maya has used the platform before, has saved payment details, and knows how the booking flow works. Nothing needs explaining."

History determines how much the system needs to explain. A returning user who knows the flow needs speed. A first-time visitor needs context at every step.

---

**6. Success**

What "done" looks like from their perspective. Not what the system logs — what they feel. The moment they know it worked and can move on.

"Maya knows she has a place, she knows when it is, and she will not forget. The confirmation must include enough detail to feel settled, and the calendar invitation is not optional."

Success drives the design of every confirmation screen and every verification step. It is the persona's definition of done, not the system's.

---

**7. Constraints**

What they cannot or will not do. The hard limits that define the outer boundary of acceptable design.

"Maya will not phone the shop to book. Will not create a new account if her login fails. Will not wait more than a few seconds for a page to load."

Constraints turn vague quality expectations into specific design requirements. Any outcome that violates them will fail in practice, no matter how well it works technically.

---

## 9. The Acid-Test Persona

Every ODD plan should include at least one acid-test persona. This is a persona at the edge of your user base — someone whose needs are non-standard, whose access is restricted, or whose context is unusual.

The acid-test persona is not the primary user. They are the stress test. If the system works correctly for the acid-test persona, it is almost certainly working correctly for everyone.

**Examples:**

A teacher who has just been hired and has not yet been assigned a cohort. Can they log in? What do they see? Can they accidentally access another teacher's data?

A student who began an application but never submitted it. Does the draft persist? Can they resume it? Is the partial application visible to teachers?

An admin who covers for a teacher who is on leave. Do they have the correct temporary permissions? Are those permissions revoked automatically when the teacher returns?

Writing acid-test personas forces you to think about states your system will definitely encounter. Ignoring them means discovering them in production.

---

## 10. Paired Personas and Data Boundaries

Many ODD systems involve paired personas: two personas who interact with the same data but from different perspectives and with different permissions.

Teacher / Student. Mentor / Mentee. Admin / Applicant. Provider / Client.

Paired personas create a specific design challenge: the same entity (an application, a booking, an assessment) must look different depending on who is viewing it.

The student's application looks like: a form they filled in, a status, a message from the teacher.
The teacher's view of the same application looks like: the form content, the student's history, action buttons to approve or reject.
The admin's view looks like: the application plus the teacher's decision plus an audit trail.

These are not three different objects. They are three views of one object, filtered by persona and permissions.

When you write outcomes for paired personas, write them separately. Outcome 4: Student submits application. Outcome 7: Teacher reviews application. Outcome 12: Admin audits application decisions. Three outcomes, one underlying entity.

The contracts exposed by Outcome 4 (the application data) are consumed by Outcomes 7 and 12. The contracts exposed by Outcome 7 (the decision data) are consumed by Outcome 12 (the audit trail) and also by the notification outcomes for the student.

---

## 11. Contracts — What They Are and Why They Matter

A contract is a named, structured piece of data that one outcome produces and another outcome consumes.

The junction box analogy: imagine your house has electricity. The wiring runs through junction boxes in the walls. Each junction box has a defined input (the cable coming from the fuse box) and defined outputs (the cables running to sockets and switches). The junction box does not generate electricity — it routes it. And it does so through a defined interface that every electrician knows.

Contracts are the junction boxes of your software system. An outcome that processes student applications produces an `application-decision` contract. It does not matter how the approval was triggered, who the teacher was, or what the UI looked like — what matters is the shape of the data that comes out. Any outcome that needs to know whether a student was approved reads the `application-decision` contract. It reads from a defined interface, not from a specific implementation.

Why does this matter for non-technical builders? Because it means you can change the implementation of one outcome without breaking every outcome that depends on it. If you redesign the teacher's approval UI, the contract it produces is unchanged. The notification system, the record system, and the reporting system continue to work because they depend on the contract, not on the UI.

---

## 12. Every Outcome Produces and Consumes

The rhythm of a connected system: every outcome, except the first, consumes something. Every outcome, except the last, produces something.

When you find an outcome that consumes nothing (except perhaps an authenticated session), it is either the first step in a user journey or it is producing data from scratch (a creation outcome: student creates an application). That is fine — but make it explicit in the contracts.

When you find an outcome that produces nothing, ask: where does the data from this experience go? If a teacher approves an application and that approval goes nowhere — does not trigger a notification, does not update the student's record, does not inform the reporting system — then what is the approval for?

Every meaningful action in a system has consequences. Those consequences must be modelled as contracts. An outcome that appears to produce nothing is either a genuine dead end (the action has no downstream effects — this is rare in real systems) or a sign that some downstream outcomes have not been written yet.

Mapping the produce/consume rhythm across your outcomes is how you discover gaps in your plan before you build.

---

## 13. The "Two Architects, One Door" Problem

Imagine two architects are each hired to design one half of a building. They work independently and produce excellent plans. When the plans are combined, there is one door where the two halves meet — but the first architect designed it at 900mm wide and the second architect designed it at 800mm wide.

Neither plan is wrong in isolation. The problem is the interface between them.

This is the most common failure mode in AI-assisted parallel development. When you ask two agents to build two outcomes simultaneously, each agent makes decisions about shared infrastructure: the database table that both outcomes read from, the API endpoint that both outcomes call, the authentication check that both outcomes rely on. Without coordination, the agents make different decisions. The outcomes do not connect.

The solution is shared contracts. Before any parallel build begins, a Coordinator agent reads all the outcomes in a phase, identifies all shared infrastructure, and publishes a canonical definition of each shared element. Both building agents read this definition and conform to it.

If Agent A builds the `applications` table with a column called `teacher_id` and Agent B builds a query that looks for `approver_id`, the handshake fails. The shared contract says the column is `teacher_id`. Both agents use `teacher_id`. The handshake succeeds.

---

## 14. Reading a Dependency Graph

A dependency graph shows the relationships between your outcomes. It answers the question: what must exist before this can be built?

Reading a dependency graph:
- An arrow from Outcome A to Outcome B means "B depends on something A produces"
- An outcome with no incoming arrows is a starting point (nothing else must be built first)
- An outcome with many incoming arrows is a dependency (many things depend on it — build it carefully and test it thoroughly)
- A cycle (A depends on B, B depends on A) is a design error — it means you have a circular dependency that cannot be resolved by linear building

The dependency graph tells you the build order. You cannot build Outcome 7 (teacher reviews application) before Outcome 4 (student submits application) because 7 consumes what 4 produces. You can build Outcomes 4 and 5 in parallel if they share no dependencies.

When you brief Claude Code or a odd-flow swarm for a phase, the dependency graph determines what gets built first and what can be built simultaneously.

---

## 15. Architecture Is Derived, Not Designed

In traditional software development, an architect designs the system structure first and developers build within it. The architecture precedes the implementation.

In ODD, it is reversed. Architecture is derived from outcomes.

When you have written all your outcomes and mapped all your contracts, the architecture is implicit in the contracts. The data layer is the set of contracts that must be persisted. The API layer is the set of contracts that must be exchanged between client and server. The authentication layer is the intersection of permissions across all personas. The notification layer is the set of contracts that trigger external communications.

You do not design these layers. You read them from your outcome documents and your dependency graph. Claude Code then implements them.

This reversal matters because it prevents gold-plating and under-building. Traditional architecture tends toward over-engineering (building infrastructure for features that do not exist yet) or under-engineering (not seeing the full scope of what will eventually be needed). ODD-derived architecture builds exactly what the outcomes require, because the outcomes are the specification.

When the plan is complete, ask Claude Code: "Read all the outcomes and contracts in this plan. Derive the architecture: what database tables are needed, what API routes, what authentication model, what external services. Do not build anything yet — just derive and describe."

---

## 16. The Build Protocol

See `build/build-protocol.md` for the full protocol. Summary:

Trellis handles all build mechanics. The domain expert follows a three-step session rhythm:

1. **Type `/trellis`** — Trellis reads the full project state from odd-flow memory and reports exactly where the build stands. No re-briefing required.

2. **Type `*build` or `*swarm`** — Trellis briefs the build AI with the full six-field specification, relevant contracts, and context from previous outcomes. The domain expert waits.

3. **Verify** — Follow the verification checklist as the persona. Record each step: pass / fail / missing. Verify the failure paths, not just the happy path. Describe any failures in domain language — what the person sees, what should happen instead. Type `confirm` when all steps pass on a single complete run.

After all outcomes in a phase are verified, Trellis runs the Integration Protocol automatically: handshake check, data flow trace, and cross-persona check. The domain expert confirms each. The phase is committed.

**The domain expert never re-briefs the AI, tracks state manually, writes handover notes, or identifies shared infrastructure.** Trellis and odd-flow memory handle all of that.

---

## 17. Common Mistakes and How to Avoid Them

**Building before the plan is complete**

The pressure to start building is real. Resist it. Every hour of upfront planning saves two hours of rework. An incomplete plan built at pace produces a system that looks finished but does not work. A complete plan built carefully produces a system that works.

The test for plan completeness: can you trace any entity (an application, a booking, a student) from its creation through every outcome that touches it to its final state? If you hit a gap — a moment where the entity should change state but no outcome handles it — the plan is incomplete.

**Describing the wrong level of detail**

Outcomes can be too abstract ("the admin manages the system") or too specific ("the admin clicks the blue button in the top-right corner of the screen"). The right level of detail is behavioural: what the persona does, what they see, what changes. Not what colour the button is, but what happens when they press it.

**Forgetting about the second persona**

Most outcomes exist in pairs. When you write the outcome for a teacher approving an application, you are describing the teacher's experience. Somewhere in that outcome, an event occurs that the student should experience. That student experience is a separate outcome. Do not collapse it into the teacher outcome.

**Treating errors as edge cases**

Error handling is not an edge case. It is a first-class concern. In any real system, users will submit incomplete forms, will attempt actions they are not permitted to perform, will encounter states they were not expecting. If your outcomes only describe the happy path, your system will behave unpredictably when anything goes wrong.

**Building outcomes in the wrong order**

The dependency graph tells you the build order. If you build Outcome 12 before Outcome 4, you are building something that depends on data that does not yet exist. The build may appear to succeed (the AI will mock the missing data) but the integration will fail. Always build in dependency order.

---

## 18. The Domain Expert's Role in the Build

When the ODD plan is approved and the build begins, the domain expert's role is clear: verify that what was built is right for real users, and describe failures in plain language.

Trellis handles the brief. When the domain expert types `*build`, Trellis reads the six-field specification from odd-flow memory and briefs the build AI with everything it needs. The domain expert does not paste context, identify contracts, or re-explain the project.

**What the domain expert does:**
- Follow the verification checklist as the persona — not as themselves reviewing a screen
- Verify the failure paths, not just the happy path
- Describe failures in domain language: what the person sees, what should happen instead
- Type `confirm` when all steps pass

**What the domain expert does not do:**
- Brief the AI with technical stack details (Trellis handles this)
- Identify what files need changing (Claude Code decides)
- Describe fixes in technical terms (describe the domain experience; Claude Code finds the cause)

**Describing failures correctly:**

Not: "The API is returning a 500 error." That is a technical description that requires knowing what an API error means.

Instead: "When I try to book an event, the page shows a generic error message. It does not tell me what went wrong or what to do next." That tells the AI what the person experiences and what should happen instead.

**The final test of a complete system:**

Run the acid-test persona. Verify the experience of the person at the edge of the user base — the first-time visitor, the customer whose payment is declined, the organiser who has no events yet. If the system handles their experience correctly, it is almost certainly handling everyone else's correctly too.
