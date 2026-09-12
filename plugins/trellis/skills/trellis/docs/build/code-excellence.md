# Trellis — Code Excellence Standard

Every line of code in an ODD project earns its place. The build agents do not produce code that merely works — they produce code that is minimal, readable, and elegant. This is not aesthetic preference. Verbose, over-abstracted code is harder to verify, harder to debug, and harder to change. In a system built by AI agents and verified by domain experts, clarity is a safety property.

---

## The Three Laws

**1. Solve it, then halve it.**

Every solution is written twice internally. The first pass makes it work. The second pass makes it minimal. Only the second pass is committed. If the second pass cannot be made significantly shorter or clearer than the first, the first pass was already good.

**2. Every line earns its place.**

No defensive code that defends against nothing. No abstractions with a single implementation. No comments that restate what the code says. No error handling for errors that cannot occur. No wrapper functions that add no logic. If a line can be removed without changing behaviour or losing clarity, remove it.

**3. Name things so well that comments are unnecessary.**

A well-named variable, function, or type is documentation. `activeStudentsInClass` does not need a comment. `data` always does. When you are tempted to write a comment, rename the thing instead.

---

## The Design-It-Twice Protocol

Before writing the final version of any function, component, or module, the build agent executes this protocol internally:

**Pass 1 — Make it work.** Write the solution that satisfies the outcome specification. Get every verification step passing. Do not optimise yet.

**Pass 2 — Make it minimal.** With the working solution in hand, rewrite it with these constraints:

- Can any function be removed by inlining its body at the call site?
- Can any variable be removed without losing readability?
- Can any conditional be replaced by a simpler expression?
- Can any abstraction layer be collapsed without creating duplication?
- Can any file be merged with another without exceeding clarity?
- Is there a standard library function that replaces custom logic?

**The output is always Pass 2.** Pass 1 is scaffolding. It is never committed.

---

## What Elegant Code Looks Like

### Good — Intermediate variables as documentation

```typescript
const activeStudents = students.filter(s => s.isActive)
const sorted = activeStudents.sort((a, b) => a.name.localeCompare(b.name))
const displayNames = sorted.map(s => `${s.firstName} ${s.lastName}`)
```

### Bad — Verbose chain with redundant comments

```typescript
// Filter to only get active students
const result = students
  .filter(student => {
    // Check if the student is active
    if (student.isActive === true) {
      return true
    }
    return false
  })
  // Sort students alphabetically by name
  .sort((a, b) => {
    return a.name.localeCompare(b.name)
  })
  // Format display names
  .map(student => {
    const displayName = `${student.firstName} ${student.lastName}`
    return displayName
  })
```

### Good — Direct return, no unnecessary wrapping

```typescript
export function canAccessDashboard(user: User): boolean {
  return user.role === "teacher" && user.isVerified
}
```

### Bad — Unnecessary conditional, unnecessary variable

```typescript
export function canAccessDashboard(user: User): boolean {
  // Check if user can access the dashboard
  const hasAccess = user.role === "teacher" && user.isVerified
  if (hasAccess) {
    return true
  }
  return false
}
```

### Good — Component with clear structure, no wrappers

```tsx
export default function StudentCard({ student }: { student: Student }) {
  const initials = student.firstName[0] + student.lastName[0]

  return (
    <div className="rounded-lg border p-4">
      <div className="flex items-center gap-3">
        <span className="bg-primary/10 text-primary rounded-full p-2 text-sm font-medium">
          {initials}
        </span>
        <div>
          <p className="font-medium">{student.firstName} {student.lastName}</p>
          <p className="text-muted-foreground text-sm">{student.email}</p>
        </div>
      </div>
    </div>
  )
}
```

### Bad — Wrapper components, unnecessary state, prop drilling

```tsx
function AvatarWrapper({ initials }: { initials: string }) {
  return (
    <span className="bg-primary/10 text-primary rounded-full p-2 text-sm font-medium">
      {initials}
    </span>
  )
}

function StudentInfo({ name, email }: { name: string; email: string }) {
  return (
    <div>
      <p className="font-medium">{name}</p>
      <p className="text-muted-foreground text-sm">{email}</p>
    </div>
  )
}

export default function StudentCard({ student }: { student: Student }) {
  const [initials, setInitials] = useState("")

  useEffect(() => {
    setInitials(student.firstName[0] + student.lastName[0])
  }, [student])

  return (
    <div className="rounded-lg border p-4">
      <div className="flex items-center gap-3">
        <AvatarWrapper initials={initials} />
        <StudentInfo
          name={`${student.firstName} ${student.lastName}`}
          email={student.email}
        />
      </div>
    </div>
  )
}
```

---

## Quantitative Guardrails

These are not style suggestions. They are constraints the build agent applies to every piece of code:

| Constraint | Limit | Why |
|---|---|---|
| Function body | 25 lines max | A function longer than 25 lines is doing more than one thing |
| Function parameters | 4 max | More than 4 parameters means the function needs a different design |
| File length | 300 lines max | Shorter than the project default — elegance demands it |
| Nesting depth | 3 levels max | Deep nesting hides logic — flatten with early returns |
| Components per file | 1 exported, 2 private max | More means the file should be split |
| Dependencies per file | 8 imports max | More means the module is coupled to too many concerns |

If a build agent finds itself exceeding these limits, it stops and restructures before continuing — not after.

---

## Anti-Patterns the Build Agent Must Refuse

**The "just in case" pattern.** Adding error handling, validation, or fallbacks for scenarios that the outcome specification does not describe. If the outcome does not mention it, it does not exist yet.

**The abstraction-first pattern.** Creating a generic utility, base class, or shared module before there are two concrete uses. Abstractions are extracted from duplication, not invented in advance.

**The config-driven pattern.** Externalising values to configuration files when they are used exactly once and will not change. A constant at the top of the file is clearer than an import from a config module.

**The wrapper-component pattern.** Creating a component that wraps another component and adds nothing — no state, no logic, no styling. If `<PrimaryButton>` renders `<Button variant="primary">`, just use `<Button variant="primary">`.

**The defensive-copy pattern.** Cloning objects, spreading arrays, or creating new references "just in case" something mutates. If nothing mutates, the copy is noise.

**The premature-types pattern.** Creating type definitions for structures that are used once, in one place, and are self-evident from context. Inline the type at the point of use.

---

## Testing Standard

Every pure-logic module — any function that takes inputs and returns outputs without side effects — MUST have a corresponding test file in `tests/` that mirrors the source path. If the source is `lib/psm/mastery.ts`, the test is `tests/lib/psm/mastery.test.ts`.

### What MUST be tested

- **Business rules:** access control checks, eligibility logic, classification functions, routing decisions
- **Data transformations:** formatting, aggregation, filtering, score calculations
- **Parsing and validation:** CSV import, regex patterns, input sanitisation, form validation
- **Safety-critical logic:** safeguarding keyword detection, content filtering, concern classification and routing
- **State machines:** plant growth levels, unlock gates, engagement level thresholds

### What is NOT unit tested

- Database queries — these are tested via the verification walkthrough
- React components — these are tested via the verification walkthrough and design verification
- LLM calls — these are tested via the verification walkthrough
- External API integrations — these are tested via the verification walkthrough

### Test quality rules

- Test the behaviour, not the implementation. Test what the function returns, not how it computes it.
- Use `it.each` for data-driven tests with multiple inputs against the same assertion.
- Use `vi.useFakeTimers()` for time-dependent logic. Clean up with `vi.useRealTimers()`.
- Use `vi.stubEnv()` for environment variable tests. Clean up with `vi.unstubAllEnvs()`.
- No mocks for things that can be tested directly. If a function is pure, test it directly.
- Every test file must pass independently — no shared state between test files.

### The testing gate

`npm test` runs before every verification walkthrough. Failing tests block verification. The domain expert never sees a system with broken business logic.

---

## How This Standard Is Enforced

1. **At build time.** The build agent reads this document before writing any code. It applies the Design-It-Twice protocol internally and outputs only the minimal version.

2. **At hook time.** The `odd-code-elegance.sh` hook runs after every file write and flags violations: files over 300 lines, functions over 25 lines, deep nesting, excessive imports. It does not block — it coaches.

3. **At verification time.** When the domain expert verifies an outcome, the code behind it has already been through two passes. The domain expert does not review code — but the code they are relying on is clean, minimal, and maintainable.

4. **At test time.** `npm test` runs after every build and before every verification. Failing tests block verification. New pure-logic functions without corresponding test files are flagged.

5. **At refactor time.** If an outcome is rebuilt after a verification failure, the rebuild starts from scratch against this standard — it does not patch the previous attempt. Existing tests must still pass after the rebuild.
