---
name: excalidraw
description: Wireframe and diagram generation for UI/design planning
version: 1.1.0
author: Trellis
source: https://github.com/excalidraw/excalidraw
tags:
  - ui-design
  - wireframing
  - planning
  - visualization
---

# Excalidraw Wireframe Generator

Generate `.excalidraw` wireframe files that can be opened in excalidraw.com or the VS Code Excalidraw extension.

## How This Skill Works

When invoked, you MUST generate a valid `.excalidraw` JSON file. This is not a documentation skill — it is a generation skill. You produce output.

## Output Format

Every invocation produces a `.excalidraw` file saved to `docs/wireframes/` in the project directory. The file is valid Excalidraw JSON that can be:
- Dragged into https://excalidraw.com for interactive editing
- Opened in VS Code with the Excalidraw extension
- Exported as PNG or SVG from either tool

## Excalidraw JSON Schema

An `.excalidraw` file is JSON with this structure:

```json
{
  "type": "excalidraw",
  "version": 2,
  "source": "trellis",
  "elements": [ ... ],
  "appState": {
    "gridSize": null,
    "viewBackgroundColor": "#0a0a0f",
    "theme": "dark"
  },
  "files": {}
}
```

### Element Types

Every element in the `elements` array has these common fields:

```json
{
  "id": "unique-id",
  "type": "rectangle|ellipse|diamond|text|line|arrow",
  "x": 0,
  "y": 0,
  "width": 100,
  "height": 50,
  "angle": 0,
  "strokeColor": "#495057",
  "backgroundColor": "#1e1e2e",
  "fillStyle": "solid",
  "strokeWidth": 1,
  "roughness": 0,
  "opacity": 100,
  "roundness": { "type": 3 },
  "seed": 1001,
  "version": 1,
  "isDeleted": false,
  "boundElements": null,
  "link": null,
  "locked": false
}
```

**Rectangle** — Use for containers, cards, buttons, inputs, navigation bars, progress bars:
- Set `roundness: { "type": 3 }` for rounded corners (cards, buttons)
- Set `roundness: null` for sharp corners (sidebars, headers)
- For buttons: small height (28-36), accent `backgroundColor`, `"transparent"` `strokeColor`
- For cards: `backgroundColor: "#313244"`, `strokeColor: "#495057"`
- For progress bars: two overlapping rectangles (background + fill)

**Text** — Use for labels, headings, body text:
```json
{
  "type": "text",
  "text": "Your label here",
  "fontSize": 14,
  "fontFamily": 1,
  "textAlign": "left",
  "verticalAlign": "top"
}
```
- Headings: `fontSize: 20-28`, `strokeColor: "#cdd6f4"`
- Body text: `fontSize: 13-14`, `strokeColor: "#cdd6f4"`
- Muted/secondary: `fontSize: 12-13`, `strokeColor: "#6c7086"`
- Accent text: `strokeColor: "#a78bfa"` (or project accent colour)

**Ellipse** — Use for avatars, status indicators:
- Equal width/height for circles

**Diamond** — Use for badges, decorative icons:
- Good for achievement/badge indicators

**Line** — Use for separators, connectors:
```json
{
  "type": "line",
  "points": [[0, 0], [200, 0]]
}
```

**Arrow** — Use for flow indicators, data flow diagrams:
```json
{
  "type": "arrow",
  "points": [[0, 0], [200, 0]],
  "startArrowhead": null,
  "endArrowhead": "arrow"
}
```

### Colour Palettes

**Dark mode (Catppuccin Mocha — default for Trellis):**
- Background: `#1e1e2e`
- Surface/cards: `#313244`
- Surface darker: `#181825`
- Border: `#495057`
- Muted surface: `#45475a`
- Text primary: `#cdd6f4`
- Text muted: `#6c7086`
- Accent purple: `#a78bfa`
- Success green: `#a6e3a1`
- Warning amber: `#fab387`
- Error red: `#f38ba8`
- Canvas background: `#0a0a0f`

**Light mode:**
- Background: `#ffffff`
- Surface/cards: `#f8f9fa`
- Border: `#dee2e6`
- Text primary: `#212529`
- Text muted: `#6c757d`
- Accent blue: `#3b82f6`
- Canvas background: `#f5f5f5`

### Layout Patterns

**Desktop (1280x800):**
- Sidebar: x=0, width=240, full height
- Main content: x=280 (240 sidebar + 40 padding)
- Content width: ~960px
- Card grid: 3 columns at 300px each with 30px gaps

**Mobile (375x812):**
- Full-width cards with 16px padding each side (x=16, width=343)
- Header bar: height=60, full width
- Bottom tab bar: y=752, height=60, full width
- Stack cards vertically with 16px gaps

**Tablet (768x1024):**
- 2-column card grid
- Collapsible sidebar or top navigation

### Wireframe Composition Rules

1. **Always include a label** above each wireframe frame identifying it (e.g., "DESKTOP — Student Dashboard (1280x800)")
2. **Use realistic content** — not "Lorem ipsum". Use domain-specific text from the specification.
3. **Show the key outcome flow** — the most important screen the persona interacts with.
4. **Include navigation** — sidebar on desktop, bottom tabs on mobile.
5. **Show data states** — progress bars should show realistic percentages, cards should have real titles.
6. **Use the project's accent colour** consistently for interactive elements (buttons, active tabs, links).
7. **Keep `roughness: 0`** for clean wireframes. Use `roughness: 1` only if a hand-drawn sketch style is requested.

### Generating Multiple Views

When generating wireframes for Step 9b (UI & Design Decision), create separate files for each design approach:

- `docs/wireframes/minimalist-dark.excalidraw` — dark mode, minimal, shadcn/ui
- `docs/wireframes/dense-dashboard-light.excalidraw` — light mode, data-dense, tables
- `docs/wireframes/minimal-accessible.excalidraw` — high contrast, large targets, WCAG AAA
- `docs/wireframes/brand-driven.excalidraw` — custom palette, domain-specific visual language

Each file should contain both desktop and mobile views side-by-side on the same canvas:
- Desktop frame at x=0
- Mobile frame at x=(desktop width + 120)

### Seed Values

Every element needs a unique `seed` value (integer). Use a simple incrementing counter:
- Desktop elements: 1001, 1002, 1003...
- Mobile elements: 2001, 2002, 2003...
- Additional views: 3001, 3002, 3003...

## Usage

When the `/excalidraw` command is invoked with arguments, parse the arguments to understand:
1. What to wireframe (which screen, which persona, which outcome)
2. Which design approach (dark/light, minimal/dense, accessible, brand-driven)
3. Which devices (desktop, mobile, tablet)

Then generate the `.excalidraw` JSON file using the schema above and save it to `docs/wireframes/[design-approach-name].excalidraw`.

After generating, tell the user:
- The file path
- How to open it: "Drag this file into excalidraw.com to view and edit interactively, or open it with the VS Code Excalidraw extension."
