# Chapter 14: The Things That Scare You

Security, authentication, and data protection are not technical mysteries. They are constraints expressed in outcome language. "This person should be able to do this. This person should not be able to do this other thing." Write them as outcomes. Verify them.

The feeling that security is "too technical" is understandable but incorrect. Security requirements are domain requirements. You know who should access what. You know which data is sensitive. That knowledge is the specification. The implementation is the AI's job.

## Key Principles

- **Every security requirement can be expressed as a pair of statements: "this person should be able to do this" and "this person should not be able to do this other thing."** A teacher should be able to see their students' grades. A student should not be able to see other students' grades. That is a security specification. Write it as an outcome. Build it. Verify it.

- **Prohibition outcomes are as important as permission outcomes.** Most people naturally write outcomes about what users can do. Security requires outcomes about what users cannot do. If you only specify what is allowed, everything else is undefined — and undefined behaviour is where vulnerabilities live.

- **Verification of security outcomes means testing the prohibition.** Log in as the wrong persona. Try to access the restricted page directly by URL. Try to see another user's data. If the system stops you, it passes. If not, describe what happened.

## Red Flags

- An outcome with no prohibition. If a permission outcome ("teachers can view grades") exists without a corresponding prohibition outcome ("students cannot view other students' grades"), the boundary is unspecified.

- No verification step for direct URL access. If your verification only checks that the navigation does not show a link, it misses the case where someone types the URL directly. Always verify that direct access is blocked, not just that the link is hidden.

- A data collection outcome with no deletion outcome. If you collect personal data, you need an outcome for deleting it. This is a legal requirement in most jurisdictions. If data comes in, there must be a way for it to go out.

- Security treated as a separate phase. Security outcomes are not an add-on. They belong in the same phase as the outcomes they constrain. The "view grades" permission outcome and the "block unauthorised grade access" prohibition outcome should be built and verified together.

## Implementation Hygiene

Domain security — who can see what, who can do what — is your responsibility. You specify it as outcomes and verify it as the persona.

Implementation hygiene is a different category. It covers things like exposed secrets, missing authentication checks, unsafe inputs, or insecure shortcuts. These are not failures of specification. They are implementation mistakes that need disciplined review during the build.

Trellis keeps those concerns visible through security baseline warnings, tests, and explicit debugging when something looks wrong. Your job is still the domain. The build workflow's job is to keep implementation quality visible without replacing verification.

## Two Types of Security Work

To be precise about the division:

**Your security work** (done during outcome writing and verification):
- Write prohibition outcomes for every permission outcome
- Verify that the wrong persona cannot access restricted data
- Verify that direct URL access is blocked, not just hidden
- Ensure data deletion outcomes exist wherever data is collected

**Implementation hygiene work** (done during build and review):
- Detect exposed credentials and secrets in code
- Detect missing or bypassed authentication checks
- Detect unsafe rendering, storage, and transport shortcuts
- Fix anything suspicious before the outcome is treated as complete

Do both. Neither domain verification nor implementation hygiene is enough alone.

## What This Means for You

For every outcome that involves access to data or actions restricted to certain personas, write the corresponding prohibition outcome. Then verify both: check that the right persona can do the thing, and check that the wrong persona cannot.

You already know who should see what in your system. Write it down in outcome format. Then verify the boundary and keep the implementation disciplined during the build.

Next: Chapter 15 shows that interface quality is a specification problem — and gives you five principles to specify it.
