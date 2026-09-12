# Trellis — Accessibility Verification Protocol

Accessibility is the practice of building software that works for everyone — including people who use keyboards instead of mice, screen readers instead of displays, or who have configured their devices to reduce motion or increase text size.

This document is written for domain experts, not developers. You do not need to understand the code to verify accessibility. You need to know what to look for and how to look for it.

---

## Why Accessibility Matters to You

### The regulatory reason

In the United Kingdom, the Equality Act 2010 requires that digital services not discriminate against disabled people. The Public Sector Bodies Accessibility Regulations 2018 mandate WCAG 2.1 AA compliance for public sector websites. Many private sector obligations follow from the Equality Act. If your platform serves students, employees, patients, or the public, you have legal obligations around accessibility.

In the United States, the ADA (Americans with Disabilities Act) has been interpreted to cover websites. Title III cases are regularly filed against organisations whose digital products are inaccessible.

Inaccessibility is not a technical edge case. It is a legal liability and a barrier to the people you are trying to serve.

### The moral reason

Approximately 1 in 5 people in the UK has a disability of some kind. Of those, many experience difficulty using software that was not designed with them in mind: people who are blind or have low vision, people with motor disabilities who cannot use a mouse, people with cognitive disabilities who struggle with complex or confusing interfaces, people with hearing loss who cannot rely on audio cues.

When you build inaccessible software in a domain you care about — education, healthcare, housing, professional services — you are excluding the people in that domain who already face the most barriers.

### The practical reason

Accessible design is better design for everyone.

- Keyboard navigation makes power users faster
- High-contrast text is easier to read in sunlight or on cheap monitors
- Clear error messages help everyone, not just people with cognitive disabilities
- Large touch targets are easier to use for everyone, not just people with motor impairments
- Captions on video benefit people in noisy environments, not just deaf viewers

When you build for accessibility, you build for edge cases. Edge cases reveal design weaknesses that affect everyone.

---

## The WCAG 2.1 AA Standard

WCAG stands for Web Content Accessibility Guidelines. It is published by the World Wide Web Consortium (W3C) and is the internationally recognised standard for web accessibility.

Version 2.1 Level AA is the standard most organisations target. It is what regulators check. It is what Trellis verifies.

WCAG is organised around 4 principles. Every guideline belongs to one.

---

## The 4 Principles

### Perceivable

Information must be presentable to users in ways they can perceive. If a user cannot see, the information must be available to them another way.

**In a real domain example:** Imagine you are building an education platform. A student who is blind uses a screen reader — software that reads the page aloud. If your student application form only communicates status through colour (red = error, green = success), that student cannot perceive the status. The principle of Perceivable requires that you also communicate status through text.

**What this means in practice:**
- Images have descriptive text alternatives
- Colour is never the only way to communicate information
- Text can be resized up to 200% without losing functionality
- Audio and video have captions or transcripts

### Operable

Interface components must be operable by users who cannot use a mouse.

**In a real domain example:** A senior student who has arthritis navigates your platform using only a keyboard. She presses Tab to move between form fields and Enter to submit. If your Submit button cannot be reached by Tab, or if a dropdown menu only works with a mouse hover, she cannot complete the application. The principle of Operable requires that every interactive element be reachable and usable by keyboard.

**What this means in practice:**
- All functionality is available from a keyboard
- No content causes seizures (no flashing more than 3 times per second)
- Users can pause, stop, or hide any moving or auto-updating content
- Users have enough time to read and use content (timeouts warn and can be extended)

### Understandable

Information and interface operation must be understandable.

**In a real domain example:** A teacher submits a course update form. It fails validation. The error message reads: "ERR_422: Field constraint violation on entity.courseTitle". The teacher does not know what to do. The principle of Understandable requires that error messages be written in language the user can act on.

**What this means in practice:**
- Text is readable and understandable (appropriate language, no unexplained jargon)
- Content behaves predictably — navigation in the same position, consistent component behaviour
- Error messages identify the problem and suggest a fix
- Form instructions appear before the form, not only in placeholder text

### Robust

Content must be robust enough to be interpreted by a wide variety of user agents, including assistive technologies.

**In a real domain example:** A student uses an older version of a screen reader that expects standard HTML semantics. If your developer built the "button" as a styled `<div>` element without ARIA roles, the screen reader cannot identify it as a button and cannot interact with it. The principle of Robust requires that you use standard, semantic HTML or supplement non-standard elements with correct ARIA roles.

**What this means in practice:**
- HTML elements are used according to their purpose (buttons for actions, links for navigation)
- All interactive components have name, role, and value exposed to assistive technologies
- Status messages are programmatically determinable (the screen reader can find them)

---

## Step-by-Step Verification Procedure

You can run all of these tests in a standard browser without any additional software (except screen reader testing, which requires a free download).

---

### Test 1 — Keyboard-Only Navigation

**What you are testing:** Can a user complete every task on this screen using only a keyboard?

**How to do it:**

1. Put your mouse to one side. Do not touch it during this test.
2. Load the page you are testing.
3. Press Tab. A visible focus ring should appear on the first interactive element.
4. Keep pressing Tab. Observe:
   - Does the focus ring move to every interactive element? (buttons, links, form fields, dropdown triggers)
   - Does it move in a logical order? (top to bottom, left to right — following the visual layout)
   - Is the focus ring clearly visible at every step?
5. When focus is on a button, press Enter or Space. Does the button activate?
6. When focus is on a link, press Enter. Does it navigate?
7. When focus is on a Select or Dropdown, press Enter to open it, arrow keys to navigate, Enter to select.
8. When a Dialog opens, confirm focus moves into the dialog. Press Tab inside the dialog — confirm focus stays within the dialog (it should not escape to the page behind). Press Escape — confirm the dialog closes and focus returns to the element that opened it.

**Pass criteria:**
- Every interactive element reachable by Tab
- Focus ring visible at every step
- All actions completable by keyboard
- Dialogs trap and return focus correctly

**Fail example:** "When I Tab to the Approve button on the Applications screen and press Enter, the confirmation dialog opens but focus stays on the button behind the dialog. I cannot reach the Confirm button without using the mouse."

---

### Test 2 — Screen Reader Spot-Check

**What you are testing:** Does the page make sense when read aloud by a screen reader?

**What you need:**
- On macOS: VoiceOver is built in. Press Cmd+F5 to turn it on.
- On Windows: NVDA is free. Download from nvaccess.org.
- On iOS: VoiceOver is built in. Settings > Accessibility > VoiceOver.

**How to do it (VoiceOver on macOS):**

1. Press Cmd+F5 to start VoiceOver.
2. Press Ctrl+Option+Right Arrow to move to the next element on the page.
3. Listen to what VoiceOver reads. Evaluate:
   - Does each form field have a clear label? ("Application date, date field" — not just "date field")
   - Do buttons have meaningful names? ("Approve application from Sarah Chen" — not just "button" or "icon")
   - Do images have appropriate descriptions? (or are decorative images skipped silently?)
   - When an error appears, does VoiceOver announce it? (without you having to navigate to it)
   - When a success toast appears, does VoiceOver announce it?
4. Press Cmd+F5 to stop VoiceOver.

You do not need to navigate the entire page. Spot-check the most important elements: forms, primary actions, status indicators, and notifications.

**Pass criteria:**
- Every form field announces its label
- Every button announces its purpose (not just "button")
- Errors and success messages are announced automatically
- Decorative images are silent

**Fail example:** "When I reach the Approve button with VoiceOver, it reads 'button'. It does not say what the button does or which application it refers to. I cannot use this without the mouse."

---

### Test 3 — Colour Contrast Check

**What you are testing:** Is the text readable for people with colour vision deficiencies or in low-contrast conditions?

**How to do it (Chrome DevTools):**

1. Open Chrome DevTools (F12 or Cmd+Option+I).
2. Click the Elements tab.
3. Click the cursor icon (Inspect Element) and click on a piece of body text.
4. In the Styles panel on the right, find the `color` property.
5. Click the colour swatch next to the value.
6. A colour picker opens. At the bottom, it shows the contrast ratio against the background.
7. The ratio must be 4.5:1 or higher for normal text, 3:1 or higher for large text (18px bold or 24px regular).

**Also check:**
- Placeholder text in form fields (this commonly fails — placeholder text is often too light)
- Muted/secondary text
- Text on coloured backgrounds (badges, buttons, alerts)
- Link text within body copy

**Pass criteria:**
- Normal text: contrast ratio 4.5:1 or above
- Large text: contrast ratio 3:1 or above
- No information conveyed by colour alone

**Fail example:** "The 'Pending' status badge uses light grey text on a light grey background. DevTools shows a contrast ratio of 2.1:1. This fails WCAG AA."

---

### Test 4 — Form Error Test

**What you are testing:** Are error messages helpful and correctly announced?

**How to do it:**

1. Find any form in the outcome you are testing.
2. Submit it empty (or with invalid values where the form requires specific formats).
3. Observe:
   - Does an error message appear?
   - Is it adjacent to the field that caused the error (not only at the top of the form)?
   - Does the error message say specifically what is wrong? ("Enter a valid email address" not "Invalid input")
   - Does the error message say how to fix it? ("Email addresses must include an @ symbol")
   - Is the user's input preserved? (they should not have to re-type the whole form)
4. Fix one field and resubmit. Does the error for that field disappear while errors on other fields remain?
5. Fix all errors and submit. Does a clear success message appear?

**Pass criteria:**
- Error messages are specific and actionable
- Errors appear adjacent to the relevant field
- Input is preserved on error
- Success is clearly confirmed

**Fail example:** "When I submit the enrolment form without filling in the student's date of birth, a red banner appears at the top saying 'Please check your form'. There is no indication which field is wrong or what I need to enter. The form is cleared."

---

### Test 5 — Mobile Touch Target Test

**What you are testing:** Are interactive elements large enough to tap reliably on a touchscreen?

**How to do it:**

1. Open Chrome DevTools and switch to responsive mode (the device icon, or Cmd+Shift+M).
2. Set the viewport to 375px wide (iPhone SE).
3. For each interactive element on the page, visually estimate whether it is at least 44x44 pixels.
4. Pay particular attention to:
   - Icon buttons (close, menu, edit, delete — these are commonly too small)
   - Checkboxes and radio buttons
   - Links within body copy
   - Table action buttons that stack on mobile

To measure precisely: right-click on an element > Inspect. In DevTools, hover over the element in the Elements panel. The dimensions appear as a tooltip.

**Pass criteria:**
- All buttons, links, and form controls: minimum 44x44px touch target
- Spacing between targets: minimum 8px (targets should not be so close that an adjacent tap is easy to trigger accidentally)

**Fail example:** "On mobile, the delete button in the student list is an X icon that is 24x24 pixels. It is next to an edit icon also 24x24 pixels with 4px between them. Both fail the touch target requirement."

---

## Common Violations and Their Fixes

### Violation: Icon-only button with no accessible label

**What it looks like:** A button that is just a trash can icon, no text.

**Why it fails:** A screen reader reads it as "button" with no further description.

**Fix in shadcn/Tailwind:**
```tsx
<Button variant="ghost" size="icon" aria-label="Delete application from Sarah Chen">
  <Trash2 className="h-4 w-4" />
</Button>
```

---

### Violation: Form field with placeholder text but no label

**What it looks like:** An input that says "Enter your email" inside the field, with no label above it.

**Why it fails:** Placeholder text disappears when the user starts typing. Screen readers may not associate it correctly. Users with cognitive disabilities lose context mid-entry.

**Fix in shadcn/Tailwind:**
```tsx
<div className="space-y-2">
  <Label htmlFor="email">Email address</Label>
  <Input id="email" type="email" placeholder="name@example.com" />
</div>
```

---

### Violation: Error message not connected to field

**What it looks like:** An error paragraph that appears near the form but is not semantically linked to the field it describes.

**Why it fails:** Screen readers cannot associate the error with the field.

**Fix:**
```tsx
<div className="space-y-2">
  <Label htmlFor="email">Email address</Label>
  <Input
    id="email"
    type="email"
    aria-describedby="email-error"
    aria-invalid={!!error}
  />
  {error && (
    <p id="email-error" className="text-sm text-destructive">
      {error}
    </p>
  )}
</div>
```

---

### Violation: Colour as the only status indicator

**What it looks like:** A table where pending rows are grey, approved rows are green, and rejected rows are red — with no text labels.

**Why it fails:** Colour-blind users cannot distinguish red from green. Screen readers cannot convey colour.

**Fix:** Use shadcn `Badge` components with both colour and text:
```tsx
<Badge variant={status === 'approved' ? 'default' : status === 'rejected' ? 'destructive' : 'secondary'}>
  {status.charAt(0).toUpperCase() + status.slice(1)}
</Badge>
```

---

### Violation: Focus not visible

**What it looks like:** Interactive elements that look the same whether focused or not.

**Why it fails:** Keyboard users cannot tell where they are on the page.

**Fix:** Never remove the `outline` without a replacement. shadcn components include `focus-visible:ring-2 focus-visible:ring-ring` by default. Do not remove these classes.

---

## The Reduce Motion Preference

Some users configure their operating system to reduce motion because they experience nausea, dizziness, or disorientation from animated interfaces. This is common in people with vestibular disorders, migraines, or certain neurological conditions.

The operating system exposes this preference as a CSS media query: `prefers-reduced-motion: reduce`.

Framer Motion reads this preference through its `useReducedMotion` hook. Use it on any animation that involves movement (translate, scale, rotate). Opacity transitions (fade in/out) are generally safe to keep even when motion is reduced.

**Pattern to use in every animated component:**
```tsx
import { motion, useReducedMotion } from 'framer-motion';

function AnimatedList({ items }) {
  const prefersReducedMotion = useReducedMotion();

  const containerVariants = {
    hidden: {},
    visible: {
      transition: {
        staggerChildren: prefersReducedMotion ? 0 : 0.05,
      },
    },
  };

  const itemVariants = {
    hidden: {
      opacity: 0,
      y: prefersReducedMotion ? 0 : 8,
    },
    visible: {
      opacity: 1,
      y: 0,
    },
  };

  return (
    <motion.ul variants={containerVariants} initial="hidden" animate="visible">
      {items.map(item => (
        <motion.li key={item.id} variants={itemVariants}>
          {item.content}
        </motion.li>
      ))}
    </motion.ul>
  );
}
```

When `prefersReducedMotion` is true, items still fade in (opacity transition) but do not slide (y transition is 0). The stagger is removed so all items appear simultaneously.

---

## Writing Accessibility Verification Steps into Outcome Documents

Every UI outcome should include accessibility verification steps alongside the standard walkthrough steps. These are not separate — they are part of the definition of done.

When writing or reviewing outcome documents, add this section after the walkthrough:

```
ACCESSIBILITY VERIFICATION

Keyboard:
- [ ] Tab from page entry through all interactive elements in logical order
- [ ] [Specific action] completable by keyboard alone
- [ ] All dialogs trap and return focus correctly

Screen reader:
- [ ] All form fields announce their labels
- [ ] [Specific button] announces "[expected announcement]"
- [ ] Success/error states announced automatically

Colour contrast:
- [ ] Body text passes 4.5:1 on all backgrounds in this view
- [ ] Status badges (Pending/Approved/Rejected) pass 4.5:1

Mobile:
- [ ] All touch targets 44px minimum at 375px viewport
- [ ] [Specific element] does not overflow on mobile

Errors:
- [ ] Error messages are specific and adjacent to the field
- [ ] User input is preserved when validation fails
```

These steps are run by the QA agent during the verification walkthrough and reported back in domain language.
