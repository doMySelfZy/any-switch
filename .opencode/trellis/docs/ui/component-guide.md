# ODD UI Component Guide
## Selecting the Right Component for Every Outcome

This guide maps common user interface needs — as described in outcome walkthroughs — to the specific shadcn/ui components that serve them best. Use this when briefing the UI agent for any outcome.

---

## How to Use This Guide

When you have a UI outcome walkthrough, read each step and ask: "What does the persona *see* here, and what can they *do*?" Match the answer to a pattern below. The UI agent will implement the component — your job is to name the right one.

---

## Data Display Patterns

### "She sees a list of [things]"
**Component: Table or Card Grid**

Use **Table** when:
- The persona needs to compare values across rows (e.g. all open bookings, all volunteers sorted by availability)
- There are 4+ columns of data
- The persona needs to sort or filter
- Example: "Rachel sees the twelve shifts sorted by fill status"

```
shadcn: Table, TableHeader, TableRow, TableCell
Add: sortable columns via @tanstack/react-table
Tailwind: table-auto w-full
```

Use **Card Grid** when:
- Each item has a visual identity (photo, icon, colour indicator)
- The persona is browsing, not comparing
- Items have a clear primary action (Book, Assign, Edit)
- Example: "Helen sees the upcoming events with cover images and remaining capacity"

```
shadcn: Card, CardHeader, CardContent, CardFooter
Tailwind: grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4
```

---

### "She sees a status indicator"
**Component: Badge**

Map domain statuses to Badge variants:
- **default** (neutral grey) — pending, draft, not started
- **secondary** (light) — in progress, assigned, scheduled
- **destructive** (red) — overdue, failed, blocked, urgent
- **outline** (bordered) — completed, verified, confirmed

```
shadcn: Badge variant="destructive" | "default" | "secondary" | "outline"
```

Never use raw colour classes for status — always use Badge so status is consistent everywhere.

---

### "She sees a summary at the top of the page"
**Component: Stat Cards**

For dashboards showing counts, totals, or key metrics:
```
shadcn: Card with large number + label + optional trend indicator
Pattern: 3–4 stat cards in a responsive grid above the main table
Tailwind: grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6
```

---

### "She sees the full record for one [thing]"
**Component: Detail Panel or Drawer**

Use **Drawer** when:
- The persona stays in context (doesn't leave the list view)
- The detail is a quick reference or quick action
- Example: "She clicks a booking and sees the full details without leaving the dashboard"

```
shadcn: Sheet (right-side drawer)
Trigger: clicking a row in the Table
```

Use **dedicated page** when:
- There's a complex workflow attached to the record
- The persona will spend significant time here
- Example: "She opens the event page and manages all its shifts"

```
Next.js: app/[resource]/[id]/page.tsx
shadcn: Tabs to organise sections of a complex record
```

---

## Form Patterns

### "She fills in [information]"
**Component: Form with react-hook-form + zod**

```
shadcn: Form, FormField, FormItem, FormLabel, FormControl, FormMessage
Validation: zod schema matching the outcome's domain rules
```

**Field type selection:**
| Domain need | Component |
|-------------|-----------|
| Short text (name, title) | Input |
| Long text (notes, description) | Textarea |
| Choose from fixed options (< 8) | Select or RadioGroup |
| Choose from fixed options (> 8, searchable) | Combobox |
| Yes/no toggle | Switch |
| Agree/acknowledge | Checkbox |
| Date selection | Calendar + Popover (shadcn DatePicker pattern) |
| File upload | custom Input type="file" with drag-drop zone |
| Multiple selections | Multi-select Combobox or CheckboxGroup |

**Always:**
- Every form field has a visible label (not just placeholder text)
- Error messages are field-specific ("Enter a valid email address" not "Invalid")
- Required fields are marked (asterisk + screen-reader text)
- Submit button is disabled while submitting (shows loading state)
- Success state is clear — either navigate away or show a success Toast

---

### "She submits and sees confirmation"
**Component: Toast (Sonner)**

```
shadcn: Toaster + toast() from sonner
Success: toast.success("Booking confirmed — Marcus has been notified")
Error: toast.error("Could not save — Helen has been alerted")
```

Toasts are for transient confirmations. Use a dedicated success screen or redirect for significant completions (payment confirmed, account created).

---

### "She fills in a multi-step process"
**Component: Step wizard with Progress indicator**

When a walkthrough has 3+ distinct stages:
```
shadcn: no built-in wizard — use Steps pattern:
  - State machine with current step index
  - Progress bar: Progress component showing % complete
  - Navigation: Back/Next buttons with validation before advancing
  - Each step is a separate component
  - State persists across steps (react-hook-form with persist)
Tailwind: divide the form into clearly named sections
```

---

## Navigation Patterns

### "She moves between sections of the platform"
**Component: Sidebar navigation**

For authenticated apps with multiple sections:
```
shadcn: no built-in sidebar — use AppSidebar pattern:
  - Fixed left sidebar on desktop (w-64)
  - Sheet (drawer) on mobile (hamburger trigger)
  - Active route highlighted
  - User profile at bottom
  - Collapsed icon-only mode on medium screens
Tailwind: hidden md:flex for desktop, Sheet for mobile
```

---

### "She moves between views of the same record"
**Component: Tabs**

```
shadcn: Tabs, TabsList, TabsTrigger, TabsContent
Max 5 tabs — more than 5 means the record needs a different structure
Keep the active tab in the URL (searchParams) so refresh preserves position
```

---

### "She returns to where she was"
**Component: Breadcrumb**

```
shadcn: Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbSeparator
Show: Home > Section > Record name
Always include a link back to the list view
```

---

## Feedback & State Patterns

### "She waits while the system processes"
**Component: Skeleton + Spinner**

Use **Skeleton** for initial page/data loading:
```
shadcn: Skeleton
Match the shape of the content that will appear
Show 3–5 skeleton items in a list, not a spinner
Tailwind: animate-pulse on Skeleton
```

Use **Spinner/Loading state** for button actions (submitting, saving):
```
Button: disabled + loading state while action runs
shadcn: Button disabled with Loader2 icon (lucide-react) + "Saving..." text
Never remove the button — disable and show progress
```

---

### "Nothing is here yet"
**Component: Empty State**

Every list view needs an empty state — what the persona sees when there's no data yet:
```
Pattern: centred illustration/icon + heading + description + primary action button
Framer Motion: subtle fade-in (opacity 0 → 1, y 10 → 0, duration 0.3s)
Content: explain WHY it's empty and WHAT to do — not just "No items found"
Example: "No shifts assigned yet — click Add Shift to get started"
```

---

### "Something went wrong"
**Component: Alert (inline) or Toast (transient)**

Use **Alert** for persistent errors that block the workflow:
```
shadcn: Alert variant="destructive"
Include: what went wrong (domain language) + what to do next
```

Use **Toast** for transient failures that don't block:
```
toast.error("Could not send confirmation — check your email settings")
```

Never show raw error messages to users. Translate every technical error into domain language before displaying it.

---

### "She needs to confirm a destructive action"
**Component: AlertDialog**

For irreversible actions (delete, cancel, archive):
```
shadcn: AlertDialog, AlertDialogTrigger, AlertDialogContent,
        AlertDialogTitle, AlertDialogDescription,
        AlertDialogAction, AlertDialogCancel
Title: "Delete this booking?"
Description: Explain the consequence in domain terms
Action button: "destructive" variant ("Yes, delete booking")
Cancel: prominent — make it easy to back out
```

---

## Animation Patterns (Framer Motion)

### When to animate

| Situation | Animation | Duration |
|-----------|-----------|----------|
| Page/route transition | Fade in (opacity 0→1) | 200ms |
| List item appears | Slide up + fade (y:10→0, opacity:0→1) | 150ms |
| Modal/drawer opens | Already handled by shadcn/Radix |  |
| Success state | Scale pulse (scale:1→1.05→1) | 300ms |
| Empty state | Gentle fade in | 250ms |
| Error shake | Horizontal shake (x: 0,−8,8,−8,8,0) | 400ms |
| Loading→content | Fade out skeleton, fade in content | 150ms |

### Always respect reduced motion
```tsx
import { useReducedMotion } from "framer-motion"

const prefersReducedMotion = useReducedMotion()

const variants = {
  initial: { opacity: 0, y: prefersReducedMotion ? 0 : 10 },
  animate: { opacity: 1, y: 0 },
}
```

### What NOT to animate
- Navigation between pages on slow connections (delays feel like bugs)
- Form field focus states (use CSS :focus-visible instead)
- Data loading (use Skeleton, not animated spinners)
- Anything that moves continuously without user interaction

---

## Responsive Breakpoints (Tailwind)

```
sm:  640px  — most phones landscape
md:  768px  — tablets, small laptops
lg:  1024px — standard laptops
xl:  1280px — desktop

Mobile-first: write base styles for 375px, use sm:/md:/lg: to add complexity
```

**Common responsive patterns:**
```
Navigation:    hidden md:flex (sidebar) | flex md:hidden (mobile menu)
Grid:          grid-cols-1 sm:grid-cols-2 lg:grid-cols-3
Text size:     text-sm md:text-base
Padding:       p-4 md:p-6 lg:p-8
Full-width CTA: w-full sm:w-auto (button fills screen on mobile)
```

---

## The UI Outcome Brief Format

When handing a UI outcome to the UI agent, structure the brief as:

```
Build Outcome [N]: [name] — UI layer

OUTCOME WALKTHROUGH: [paste full walkthrough]

COMPONENT SELECTIONS:
Screen 1 — [name]:
  Layout: [dashboard / form / list+detail / wizard]
  Components: [list from this guide]
  Data from contracts: [which fields from which upstream outcome]
  Mobile behaviour: [how this adapts at 375px]

Screen 2 — [name]:
  [same structure]

VERIFICATION (UI-specific additions):
  [ ] Verify at 375px — all content readable, no horizontal scroll
  [ ] Keyboard tab order makes sense on every screen
  [ ] Loading states visible for every async action
  [ ] Empty states present for every list
  [ ] Error states present for every form

UI STANDARDS:
  - shadcn/ui components only (no custom component library)
  - Tailwind classes only (no inline styles)
  - Framer Motion for: [list the specific animations needed]
  - WCAG 2.1 AA required
  - Reduce motion respected via useReducedMotion()
```
