---
# EXAMPLE OUTCOME — Read this, then delete this file and create your own
# Use /trellis → *outcome in Claude Code to write outcomes interactively
# Review Status: APPROVED (example)
---

# Outcome 1: Rachel Fills an Open Volunteer Shift

---

## Field 1: Persona
**Rachel** — the volunteer coordinator. Managing on her phone between other tasks, maximum 2 taps to start any action, zero tolerance for slowness during urgent windows.

See: `docs/personas/rachel-volunteer-coordinator.md`

---

## Field 2: Trigger
It is Wednesday afternoon. Rachel has just received a WhatsApp message from a volunteer cancelling their Saturday morning shift at the Northside site. The event is 3 days away and the shift has a minimum of 2 volunteers — it now has 1. Rachel needs to assign a replacement before Thursday afternoon, when she sends the pre-event confirmation messages.

---

## Field 3: Walkthrough

Rachel opens the app on her phone. She sees a notification badge on the Shifts section — the system has already flagged the cancellation and marked the Saturday morning shift at Northside as understaffed.

She taps the notification. The app opens directly to the affected shift: "Saturday 14 June — Northside Morning (8:00–13:00)". She can see it currently has 1 volunteer confirmed (David Chen) and needs a minimum of 2. A red badge shows "1 below minimum."

She taps "Find a volunteer." The app shows her a filtered list of 7 volunteers who are:
- Available on Saturday morning (they haven't declined or been assigned elsewhere)
- Certified for food handling (required for this shift)
- Based within 5 miles of the Northside site

Each volunteer card shows their name, how many shifts they've done this year, their last shift date, and a green/amber/red availability indicator. Rachel can see that Amara Osei is available and local — she taps her name.

Amara's full profile slides in from the right. Rachel can see her certification dates, her availability pattern, and a note that she's done 3 Northside shifts before. Rachel taps "Assign to shift."

A confirmation dialog appears: "Assign Amara Osei to Saturday 14 June, Northside Morning?" with Confirm and Cancel buttons. Rachel taps Confirm.

The shift now shows 2 of 2 volunteers. The understaffed badge is gone. Amara receives an automatic WhatsApp message and email: "Hi Amara — you've been assigned to the Northside Morning shift on Saturday 14 June, 8:00–13:00. Parking is available at the rear entrance. Your shift leader is David Chen (07700 900123). Reply CONFIRM to accept or DECLINE if you can't make it."

Rachel sees a banner: "Assignment sent. Amara has 48 hours to confirm." The shift card now shows Amara's status as "Pending confirmation."

If Amara does not respond within 48 hours, Rachel receives a notification: "Amara has not confirmed the Saturday shift. Tap to find another volunteer or send a reminder." The shift returns to amber (understaffed warning) until Amara confirms or Rachel finds an alternative.

If Amara declines, Rachel receives an immediate notification and the shift returns to the "Find a volunteer" state, with Amara removed from the available list for this shift.

On Saturday morning, Rachel opens the event roster. She sees both David and Amara confirmed, with their arrival time, emergency contact, and certification status. She does not see their home addresses.

---

## Field 4: Verification

1. Log in as Rachel. Open the Shifts section. Confirm there is a notification badge for the cancelled shift.
2. Tap the notification. Confirm the app opens directly to the affected shift (not a generic shifts list).
3. Confirm the shift shows "1 below minimum" in red.
4. Tap "Find a volunteer." Confirm the list shows only volunteers who are (a) available Saturday morning, (b) food-handling certified, and (c) within 5 miles of Northside. Assign a test volunteer who fails one of these criteria and confirm they do NOT appear.
5. Tap a volunteer. Confirm the profile shows certifications, availability pattern, and shift history — but NOT their home address.
6. Assign a volunteer. Confirm the confirmation dialog appears before the assignment is made.
7. Confirm the assignment. Confirm the shift now shows 2 of 2 and the red badge is gone.
8. Confirm the assigned volunteer receives a WhatsApp/email notification with shift time, location, parking, and shift leader contact.
9. Wait 48 hours (or simulate in test data). Confirm Rachel receives a reminder notification if the volunteer has not confirmed.
10. Simulate a decline. Confirm Rachel is notified immediately and the shift returns to understaffed state.
11. Open the event roster as Rachel on Saturday morning. Confirm both volunteers appear with confirmation status and certifications. Confirm home addresses are NOT visible.

---

## Field 5: Contracts Exposed

**Confirmed volunteer assignment:**
- Volunteer (name, certification status, shift history)
- Shift (date, time, site, shift leader)
- Assignment status (pending / confirmed / declined)
- Notification sent timestamp
- Confirmation deadline

Consumed by:
- Outcome 3: Event roster generation (site managers view)
- Outcome 4: Volunteer hours tracking (Rachel's monthly report)
- Outcome 5: Certification expiry alerts (uses certification dates from assignments)

**Updated shift fill status:**
- Shift ID, site, date
- Current fill count vs minimum
- Understaffed flag (true/false)

Consumed by:
- Outcome 2: Event dashboard (Rachel sees all shifts across all events at a glance)
- Outcome 6: Pre-event confirmation send (only triggers when all shifts are filled)

---

## Field 6: Dependencies

- **Volunteer profile with certifications** — from Outcome 0: Volunteer Registration. A volunteer must exist in the system with their certification status before they can appear in the filtered assignment list.
- **Shift definition** — from shared infrastructure (Phase A): events and their shifts must be created before they can be filled. The Saturday shift at Northside must exist as a record.
- **Availability tracking** — from Outcome 0: Volunteer Registration captures availability patterns. This outcome consumes that data for the filter.
- **Notification service** — Phase A shared infrastructure: WhatsApp/email sending capability must be set up before assignment notifications can be sent.

No dependency on any other outcome in Phase B — this outcome can be built as soon as Phase A and Outcome 0 (Volunteer Registration) are verified.

---

*This example shows what a complete, approved outcome looks like. Note: specific (Rachel, not "the coordinator"), sequential walkthrough (each action triggers the next), failure paths included (non-response, decline), verification steps physically performable in a browser, contracts named in domain language.*
