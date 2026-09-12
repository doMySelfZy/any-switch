# Trellis — UI Excellence Layer

The UI is not the system. The UI is paint on top of the system.

This is the most important sentence in this document. Every choice in this guide flows from it. Your business logic — the rules about who can approve what, how fees are calculated, what triggers a notification — lives in your outcomes, your contracts, and your data layer. The UI's job is to make those capabilities accessible, legible, and pleasant to use. It contains zero business logic of its own.

When you hold this distinction clearly, UI work becomes simpler. You are not designing a system. You are designing a window into a system that already exists.

---

## The Default Stack

Trellis uses a specific set of tools for all UI work. These are not arbitrary choices — each one was selected for a reason that matters to non-technical builders.

### Next.js App Router

Next.js renders pages on the server and sends them to the browser pre-built. This means users see content quickly, and search engines can index public pages. The App Router is the current architecture — it uses React Server Components to keep heavy logic on the server and lightweight interaction code in the browser.

You do not need to understand React Server Components to use them. You need to know: if a component reads data from your database, it is a server component. If a component responds to user clicks or typing, it is a client component. Claude Code will handle the distinction.

### TypeScript

TypeScript is JavaScript with type annotations. Types are descriptions of the shape of your data — "this value is a string", "this object has a name and an email and a boolean called isApproved". TypeScript catches contradictions before they reach users. If a UI component expects a student name and receives an empty value, TypeScript flags it during build, not at 2am when a user reports a bug.

### Tailwind CSS v4

Tailwind provides pre-built utility classes that you apply directly to HTML elements. Instead of writing `color: blue; font-size: 14px; margin: 8px;`, you write `text-blue-600 text-sm m-2`. The classes are consistent across the project — `m-2` always means 8px, everywhere. This creates visual consistency without needing a custom CSS file.

Tailwind v4 uses CSS variables under the hood, which means your design tokens (colours, sizes, spacing) are defined once and flow through the entire UI automatically.

### shadcn/ui

shadcn/ui provides a library of beautiful, accessible components — buttons, forms, tables, dialogs, navigation — that you copy into your project and own directly. This is different from most UI libraries, which are black boxes you import and cannot modify. With shadcn, the component code lives in your codebase. You can change anything.

Each shadcn component is built on Radix UI primitives, which means keyboard navigation, focus management, and ARIA attributes are handled correctly without any extra work.

### Radix UI

Radix is the engine beneath shadcn. It handles the hard parts of accessible interactive components: dropdown menus that close when you press Escape, dialogs that trap focus correctly, sliders that respond to arrow keys. You will rarely interact with Radix directly — shadcn exposes it through a cleaner interface.

### Framer Motion

Framer Motion is an animation library for React. It makes the interface feel alive — list items that slide in when they appear, buttons that give subtle feedback when clicked, page transitions that guide the eye. Used well, animation communicates meaning: a success state, a loading state, an error state. Used poorly, animation is noise.

The rule for when to use Framer Motion is in the component selection guide below.

### How they work together

```
User visits a page
  → Next.js App Router renders the page on the server
  → Data from Drizzle/PostgreSQL is fetched
  → Page is sent to browser as HTML

User sees the page
  → shadcn/ui components provide the visual structure
  → Tailwind classes provide the styling
  → Design tokens (colours, spacing) flow from tailwind.config

User interacts
  → Client components handle interaction
  → Framer Motion animates state changes
  → shadcn/Radix handles keyboard and focus behaviour
```

---

## Design Token System

Design tokens are the single source of truth for your visual language. When you define a primary colour as `hsl(221 83% 53%)`, every button, every link, every highlighted element in your entire application uses that colour. Changing one value changes everything.

### Setting up tokens in globals.css

```css
@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 222.2 84% 4.9%;

    --primary: 221.2 83.2% 53.3%;
    --primary-foreground: 210 40% 98%;

    --secondary: 210 40% 96.1%;
    --secondary-foreground: 222.2 47.4% 11.2%;

    --accent: 210 40% 96.1%;
    --accent-foreground: 222.2 47.4% 11.2%;

    --destructive: 0 84.2% 60.2%;
    --destructive-foreground: 210 40% 98%;

    --muted: 210 40% 96.1%;
    --muted-foreground: 215.4 16.3% 46.9%;

    --border: 214.3 31.8% 91.4%;
    --input: 214.3 31.8% 91.4%;
    --ring: 221.2 83.2% 53.3%;

    --radius: 0.5rem;
  }

  .dark {
    --background: 222.2 84% 4.9%;
    --foreground: 210 40% 98%;
    /* dark mode values mirror the above */
  }
}
```

### Typography scale

Use a maximum of 4 sizes and 2 weights across your entire UI. Fewer choices create more coherent designs.

```
text-sm    — 14px — body copy, metadata, helper text
text-base  — 16px — default body, form labels
text-lg    — 18px — section headings, card titles
text-xl    — 20px — page headings
text-2xl   — 24px — primary page titles (use sparingly)

font-normal   — body copy, descriptions
font-semibold — headings, labels, emphasis
```

Resist the temptation to add `text-xs` for tiny metadata or `text-3xl` for hero text. Each exception weakens the rhythm of the design.

### Spacing

Tailwind's spacing scale is based on 4px increments. Use it consistently:

```
p-1  / m-1  — 4px   — tight internal padding (badges, chips)
p-2  / m-2  — 8px   — small gaps between related elements
p-4  / m-4  — 16px  — standard internal padding (cards, form fields)
p-6  / m-6  — 24px  — section separation
p-8  / m-8  — 32px  — major layout divisions
p-12 / m-12 — 48px  — page-level breathing room
```

### Border radius

Use `rounded-md` (6px) consistently for all interactive elements: buttons, inputs, cards, dialogs. Use `rounded-full` only for avatars and status indicators. Do not mix radius values — inconsistency reads as carelessness.

---

## Component Selection Guide

When a UI outcome is being built, every element in the walkthrough should be mapped to a specific component before building begins. This prevents Claude Code from inventing components that do not match your design system.

### Data display

| Need | Component | Notes |
|------|-----------|-------|
| List of records | `Table` (shadcn) | Use for 5+ fields per record |
| Summary view | `Card` (shadcn) | Use for 2-4 fields, or when visual grouping matters |
| Status indicator | `Badge` (shadcn) | Pending / Active / Cancelled / Complete |
| Numeric summary | `Card` with large text | Dashboard stats, counts, totals |
| Hierarchical data | Nested `Card` or `Accordion` | Parent-child relationships |

### Forms

| Need | Component | Notes |
|------|-----------|-------|
| Short text | `Input` (shadcn) | Name, email, reference number |
| Long text | `Textarea` (shadcn) | Descriptions, notes, feedback |
| Choose one | `Select` or `RadioGroup` (shadcn) | Use Select for 5+ options |
| Choose multiple | `Checkbox` group (shadcn) | Use for 2-6 options |
| Toggle a setting | `Switch` (shadcn) | Binary on/off, not for data entry |
| Date | `DatePicker` (shadcn + react-day-picker) | Never use a plain text input for dates |
| File upload | `Input type="file"` styled | Always show file size limits |

### Navigation

| Need | Component | Notes |
|------|-----------|-------|
| Primary navigation | `Sidebar` (shadcn) | For authenticated app shells |
| Secondary navigation | `Tabs` (shadcn) | Within a page, between related views |
| Location context | `Breadcrumb` (shadcn) | For deep hierarchies (3+ levels) |
| User menu | `DropdownMenu` (shadcn) | Profile, settings, sign out |
| Page list | `NavigationMenu` (shadcn) | For marketing/public pages |

### Feedback and actions

| Need | Component | Notes |
|------|-----------|-------|
| Non-blocking notification | `Toast` (sonner) | Success, error, info after actions |
| Blocking message | `Alert` (shadcn) | Warning that requires attention before proceeding |
| Confirmation required | `Dialog` (shadcn) | Destructive actions, irreversible changes |
| Primary action | `Button variant="default"` | One per view |
| Secondary action | `Button variant="outline"` | Supporting actions |
| Destructive action | `Button variant="destructive"` | Delete, cancel, revoke |
| Disabled action | `Button disabled` | When action is not yet available |

### Loading states

| Situation | Use |
|-----------|-----|
| Content is loading | `Skeleton` (shadcn) — mirrors the shape of the content |
| Action is processing | `Button` with `Loader2` spinner icon, disabled |
| Page is transitioning | Framer Motion fade between pages |
| List is loading | Skeleton rows matching expected content shape |

Never use a blank white screen for loading. Never show a spinner with no surrounding context — the user cannot tell what is loading or how long it will take.

---

## When to Use Framer Motion

Framer Motion is a powerful tool that is easy to overuse. The test is: does this animation communicate meaning, or is it just movement?

**Use Framer Motion for:**

- Page transitions — a subtle fade-and-slide guides the eye from one view to the next
- List item entry — items that appear in a list one by one (staggered children) communicate that data is loading progressively
- Success states — a checkmark that scales in confirms the action completed
- Empty states — an illustration that fades in softens the jarring appearance of nothing
- Sidebar open/close — smooth panel transitions feel intentional, not mechanical

**Do not use Framer Motion for:**

- Hover effects on standard buttons (CSS transitions are sufficient and faster)
- Constant ambient animation (rotating elements, pulsing backgrounds)
- Anything that distracts from the content

**Always respect reduced motion:**

```tsx
import { useReducedMotion } from 'framer-motion';

const shouldReduceMotion = useReducedMotion();

const variants = {
  hidden: { opacity: 0, y: shouldReduceMotion ? 0 : 8 },
  visible: { opacity: 1, y: 0 },
};
```

Users with vestibular disorders, epilepsy, or motion sensitivity may have configured their operating system to reduce motion. Framer Motion's `useReducedMotion` hook reads this preference. Always use it for any animation involving movement (translate, scale). Opacity-only transitions are generally safe to preserve.

---

## Mobile-First Verification Protocol

Every UI outcome is verified at mobile size first. If it works at 375px (the width of an iPhone SE, the smallest common smartphone), it works everywhere. If it only works at desktop width, it is not finished.

### Verification sequence

**1. Open browser DevTools and set viewport to 375px.**

In Chrome: press F12, then click the device icon in the top-left of DevTools. Set width to 375, height to 812.

**2. Run the mobile checklist:**

- [ ] All text is readable without zooming. No text smaller than 14px (Tailwind `text-sm`).
- [ ] All buttons and interactive elements have a minimum touch target of 44x44px.
- [ ] Form fields are large enough to tap accurately. Inputs at least 44px tall.
- [ ] No content is cut off or hidden behind other elements.
- [ ] No horizontal scroll. The page fits within 375px width.
- [ ] Navigation is accessible. On mobile, primary navigation should be in a bottom bar or a hamburger menu — never in a top sidebar that requires precise mouse targeting.
- [ ] The most important action on each screen is reachable with the right thumb (bottom half of screen).

**3. Verify at 768px (tablet).**

The layout may reflow. Two-column layouts may appear here that were single-column on mobile. Verify that the reflow is intentional and that content does not overlap.

**4. Verify at 1280px (desktop).**

Full layout. Sidebar navigation visible if applicable. Multi-column data tables visible. Verify that content does not stretch uncomfortably wide — apply `max-w-7xl mx-auto` to contain content on very wide screens.

---

## WCAG 2.1 AA Checklist

Accessibility is not a nice-to-have. It is a legal requirement in most jurisdictions and a quality indicator. Inaccessible software tells users they were not thought of. The shadcn/Radix stack handles much of this automatically — but you must verify it.

Run this checklist after every UI outcome verification:

### Colour contrast

- [ ] Normal text (under 18px): minimum 4.5:1 contrast ratio against background
- [ ] Large text (18px or larger): minimum 3:1 contrast ratio
- [ ] Interactive elements (buttons, links): minimum 3:1 against adjacent colours
- Check using the browser's built-in accessibility inspector or https://webaim.org/resources/contrastchecker/
- The default shadcn colour tokens pass AA when used as intended. Violations occur when custom colours are introduced without checking contrast.

### Keyboard navigation

- [ ] Tab through every interactive element on the page in a logical order (top to bottom, left to right)
- [ ] Every interactive element is reachable by Tab
- [ ] No element traps focus (pressing Tab always moves to the next element)
- [ ] Modals and dialogs trap focus correctly (Tab cycles within the dialog only while it is open)
- [ ] Press Escape to close any overlay — dialogs, dropdowns, drawers

### Focus indicators

- [ ] A visible focus ring appears on every focused element
- Never set `outline: none` without providing an alternative focus indicator
- shadcn components use `ring` classes for focus — do not remove these

### Screen reader basics

- [ ] All images have meaningful `alt` text, or `alt=""` if decorative
- [ ] Icon-only buttons have `aria-label` attributes (e.g. `aria-label="Close"` on an X button)
- [ ] Form inputs have associated labels (either `<label>` elements or `aria-label`)
- [ ] Headings follow a logical hierarchy (one `h1` per page, `h2` for sections, `h3` for subsections)
- [ ] Status messages (loading, success, error) are announced to screen readers using `role="status"` or `aria-live`

### Error messages

- [ ] Error messages are specific: "Enter a valid email address" not "Invalid input"
- [ ] Error messages appear adjacent to the field that caused the error
- [ ] Errors are announced to screen readers (use `aria-describedby` to connect field to error message)
- [ ] The form does not clear on error — the user's input is preserved

### Form success

- [ ] A clear confirmation appears after a successful form submission
- [ ] The confirmation states what happened in domain language: "Application submitted" not "Success"
- [ ] If the user is redirected after success, the new page makes the result obvious

---

## Layout Architecture (Defined in Step 9c, Enforced in Every Build)

Every project with more than 3 authenticated pages MUST define a layout architecture before the first line of UI code is written. This is not optional. Without it, build agents create isolated pages with ad-hoc navigation, and the platform feels like a disconnected collection of forms rather than a cohesive product.

The layout architecture is defined during planning (Step 9c of the Build Planner) and recorded in CLAUDE.md. It specifies:

1. **An app shell** — a shared layout component wrapping all authenticated pages, providing persistent navigation, header, and consistent content structure
2. **A navigation structure** — sidebar, top bar, bottom tabs, or a combination. With specific items, icons, routes, and conditional visibility rules
3. **A responsive breakpoint strategy** — what the layout does at desktop (1280px+), tablet (768px–1279px), and mobile (<768px)
4. **A page relationship map** — how every outcome's screen connects to every other screen. No dead ends allowed — every page must be reachable within 2 taps using only the platform's navigation

Build agents MUST read the layout architecture from CLAUDE.md before implementing any page. Every page renders inside the app shell. No page may render its own navigation. If a build agent creates a page that is a dead end or ignores the app shell, it fails design verification.

**The app shell is the first thing built in Phase A** — before any outcome-specific page. It is part of the shared infrastructure, just like authentication and the database schema. All subsequent pages render inside it.

---

## Outcome-to-Component Mapping (Required Before Build)

Before the session brief for each phase is generated, map every outcome's screens to specific components and layout patterns. This mapping is included IN the session brief — build agents read it before writing a line of code.

This is not optional. Without it, build agents choose components ad-hoc, creating visual inconsistency across outcomes. The mapping takes 10 minutes per outcome and prevents hours of rework.

**Step 1: Read the walkthrough and list every distinct screen.**

A screen is any distinct view the persona can be on. The list page is one screen. The detail page is one screen. The create form is one screen.

**Step 2: For each screen, identify the layout pattern.**

- Dashboard: primary metrics + activity feed + quick actions
- Form: heading + fields + submit + validation feedback
- List + detail: filterable/sortable list on one side, selected item detail on the other
- Wizard: multi-step form with progress indicator, one step per screen
- Empty state: illustration + explanation + primary action

**Step 3: For each interactive element, select the shadcn component.**

Go through the walkthrough line by line. Every button, every field, every status indicator maps to a specific component from the selection guide above.

**Step 4: Map Contracts Exposed to UI components.**

The data this outcome makes available (Contracts Exposed) needs to be displayed somewhere. Map each piece of data to the component that displays it. This ensures Claude Code knows what data needs to come from where.

**Step 5: Write the UI brief.**

---

## Example UI Brief Format

Use this format when briefing Claude Code for a UI outcome:

```
Build the UI for Outcome: [Outcome Name]

PERSONA: [Persona name and brief description]

SCREENS:
1. [Screen name] — layout pattern: [list/form/dashboard/etc]
2. [Screen name] — layout pattern: [list/form/dashboard/etc]

COMPONENT MAPPING:
- Application list: shadcn Table with columns [Name, Date, Status, Actions]
- Status column: shadcn Badge, variant based on status value (pending=secondary, approved=default, rejected=destructive)
- Approve button: shadcn Button variant="default", opens shadcn Dialog for confirmation
- Reject button: shadcn Button variant="destructive", opens shadcn Dialog with Textarea for reason
- Success feedback: sonner toast, "Application approved" / "Application rejected"
- Loading state: Skeleton rows while data fetches

DATA BINDING:
- Table rows bind to: applications[] from Contracts Consumed (application-list contract)
- Approve action calls: POST /api/applications/[id]/approve
- Reject action calls: POST /api/applications/[id]/reject with reason

TAILWIND CONVENTIONS:
- Page wrapper: max-w-7xl mx-auto px-4 sm:px-6 lg:px-8
- Card padding: p-6
- Section spacing: space-y-6
- Table header: text-sm font-semibold text-muted-foreground

MOBILE REQUIREMENTS:
- Table becomes a card list on mobile (< 640px)
- Each card shows: applicant name, date, status badge, and action buttons stacked
- Touch targets minimum 44px

ANIMATION:
- Table rows fade in using Framer Motion staggered children (0.05s delay between rows)
- Dialog open/close uses shadcn default animation (already handled)
- Respect useReducedMotion — disable stagger if preference set

ACCESSIBILITY:
- Table caption: "Student applications — [current filter]"
- Approve/reject buttons: aria-label includes applicant name ("Approve application from [name]")
- Dialog: focus moves to first interactive element when opened
- Toast: role="status" for success messages

Full walkthrough for reference:
[PASTE FULL OUTCOME WALKTHROUGH HERE]
```

This brief gives Claude Code everything it needs: what to build, what to use for each element, how the data connects, how to handle mobile, how to handle animation, and how to handle accessibility. It leaves nothing to interpretation.
