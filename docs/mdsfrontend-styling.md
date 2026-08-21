# mdsfrontend — Styling / Design System Reference

Extracted from [demetri-vlrk/mdsfrontend](https://github.com/demetri-vlrk/mdsfrontend).
A dark-mode marketing/campaign dashboard built with **React 19 + Tailwind CSS v4 + Geist Sans**.
The entire theme is defined via Tailwind v4's `@theme` block in `src/index.css`; there is no
separate config file. Components consume the tokens through utility classes (`bg-background`,
`text-muted-foreground`, `border-border`, etc.).

---

## 1. Foundations

### Stack
- **Framework:** React 19, react-router-dom 7 (routing only, no state lib)
- **Styling:** Tailwind CSS v4 (`@tailwindcss/vite`), tokens declared in `@theme`
- **Font:** Geist Sans (`@fontsource/geist-sans`, weights 400/500/600)
- **Icons:** `lucide-react` (rendered at `size-4` = 16px almost everywhere)
- **Base:** `body` = `--color-background` bg, `--color-foreground` text, antialiased

### Design character
Flat, dark, high-contrast, near-monochrome. Sharp corners dominate (cards/inputs use
1–4px radii; surfaces are often square). Accent color is largely reserved — most UI is
grayscale with white-on-black; a violet **brand** ramp and a blue chart line exist for accents,
plus gradient glow effects for hero/marketing moments.

---

## 2. Color Tokens

Primary (active) palette — used by the shipped UI:

| Token | Value | Usage |
|---|---|---|
| `--color-background` | `#0a0a0a` | App canvas, top nav, sidebar |
| `--color-foreground` | `#fafafa` | Default text / icons |
| `--color-muted-foreground` | `#a3a3a3` | Secondary text, placeholders |
| `--color-border` | `#404040` | Default 1px borders / dividers |
| `--color-border-strong` | `#737373` | Emphasized borders |
| `--color-input` | `rgba(255,255,255,0.05)` | Input field fill |
| `--color-primary` | `#f5f5f5` | Primary button bg (near-white) |
| `--color-primary-foreground` | `#0a0a0a` | Text on primary buttons |
| `--color-secondary` | `#262626` | Secondary button / badge / panel fill |
| `--color-secondary-foreground` | `#f5f5f5` | Text on secondary |
| `--color-accent` | `#171717` | Elevated surfaces, avatars, modals |
| `--color-card` | `#171717` | Card fill |
| `--color-destructive` | `#9e4042` | Danger / delete |
| `--color-chart-line` | `#3f8dff` | Chart lines / blue accents |

Sidebar-specific:

| Token | Value |
|---|---|
| `--color-sidebar` | `#0a0a0a` |
| `--color-sidebar-foreground` | `#d4d4d4` |
| `--color-sidebar-muted` | `#737373` (section labels) |
| `--color-sidebar-accent` | `#171717` |

**Translucent-white convention:** interactive surfaces layer white over the dark bg instead of
using solid grays. Common values: `bg-white/5` (rest), `bg-white/10` / `bg-white/20` (hover).
This is the dominant hover pattern across the app.

### Brand ramp (violet) + neutral gray ramp
Imported from a Figma "mbs-frontend" token set. The **brand** ramp is the intended accent color;
a full `gray-0…900` ramp and semantic aliases (`bg-canvas`, `fg-muted`, `accent-primary`,
status colors) are also defined for future/secondary surfaces.

```
brand-50  #f5f3ff   brand-400 #a78bfa   brand-700 #5b21b6
brand-100 #ede9fe   brand-500 #8b5cf6   brand-800 #4c1d95
brand-200 #ddd6fe   brand-600 #7c3aed   brand-900 #312e81
brand-300 #c4b5fd
```

Semantic status tokens: `status-errorfg #ef4444`, `status-successfg #22c55e`,
`status-warningfg #f59e0b` (each with a dark `*bg` companion).
Also `lime-400 #a3e635` / `lime-950 #1a2e05` for a highlight accent.

---

## 3. Typography

Custom Tailwind text scale (modular ~1.25 ratio) defined as `--text-*` tokens:

| Class | Size | Weight | Line-height | Tracking |
|---|---|---|---|---|
| `text-h1` | 48.8px | 700 | normal | -0.02em |
| `text-h2` | 39px | 700 | 1.1 | -0.02em |
| `text-h3` | 31.25px | 600 | 1.25 | 0 |
| `text-h4` | 25px | 600 | 1.25 | 0 |
| `text-body` | 16px | 400 | 1.5 | 0 |
| `text-body-sm` | 14.4px | 400 | 1.5 | 0 |
| `text-caption` | 12.8px | 500 | 1.7 | 0.03em |

In practice components mostly use raw Tailwind sizes with **tight negative tracking** on
headings, e.g. page titles: `text-5xl leading-[48px] font-semibold tracking-[-1.5px]`,
section titles: `text-[30px] font-semibold tracking-[-1px]`, card titles:
`text-2xl leading-[28.8px] font-semibold tracking-[-1px]`.

---

## 4. Radius Tokens

Small / sharp by design:

| Token | Value | Use |
|---|---|---|
| `--radius-surface` | 2px | Cards / surfaces |
| `--radius-control` | 4px | Inputs / controls |
| `--radius-badge` | 9999px | Pills / chips |
| `--radius-avatar` | 9999px | Avatars |
| `--radius-table` | 0 | Tables |

Note: many components intentionally use **no radius** (square cards/panels with just a border)
or `rounded-md`/`rounded-lg` for nav items and search. Pills use `rounded-full`.

---

## 5. Component Patterns

### App shell (every page)
```
<div className="min-h-svh bg-background">
  <TopNav />                        {/* h-16, border-b */}
  <div className="flex">
    <Sidebar />                     {/* w-[259px], border-r */}
    <main className="flex min-h-[calc(100svh-4rem)] flex-1 flex-col items-start">
      <div className="... px-8 py-10"> ... </div>
    </main>
  </div>
</div>
```
- Top nav height is `4rem` (h-16); main/sidebar heights subtract it via `calc(100svh-4rem)`.
- Page content padding: `px-8 py-10`.

### Buttons
| Variant | Classes |
|---|---|
| **Primary** | `flex min-h-9 items-center justify-center gap-2 bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90` |
| **Secondary** | `bg-secondary px-4 py-2 text-sm font-medium text-secondary-foreground hover:bg-secondary/80` |
| **Outline** | `border border-border px-4 py-2 text-sm font-medium text-foreground hover:bg-white/5` |
| **Ghost/glass** | `border border-border bg-white/10 px-4 py-2 text-sm font-medium text-foreground shadow-xs hover:bg-white/20` |
| **Icon button** | `flex min-h-9 min-w-9 items-center justify-center rounded-lg p-2 text-foreground hover:bg-white/5` |

Note buttons are typically **square** (no `rounded-*`) except icon buttons (`rounded-lg`).
`gap-2` between icon and label; icons at `size-4`.

### Chip / Pill (`Chip.tsx`)
```
min-h-8 rounded-full px-3 py-1.5 text-sm font-medium shadow-xs
  active  → bg-primary text-primary-foreground
  rest    → bg-white/5 text-foreground hover:bg-white/10
```

### Badge (`SectionHeader`)
```
min-h-6 rounded-full bg-secondary px-2 py-[3px] text-xs font-medium text-secondary-foreground
```

### Card (square, bordered)
```
flex flex-col items-start border border-border p-8      // ProjectCard / generic
```
Image cards overlay a bottom gradient scrim: `bg-gradient-to-b from-black/0 from-50% to-black`,
with content in a `relative` layer above an `absolute inset-0` image.

### FormCard (`FormCard.tsx`)
```
flex flex-1 flex-col gap-5 border border-border px-5 py-16
  title: text-2xl font-semibold tracking-[-1px] text-white  (+ Info icon, size-4, muted)
  desc:  text-xs leading-4 text-muted-foreground
```

### Sidebar (`Sidebar.tsx`)
- Container: `w-[259px] border-r border-border bg-sidebar px-4 py-3`, `h-[calc(100svh-4rem)]`
- Search box: `rounded-lg border border-border bg-white/5 px-3 py-[7.5px] shadow-xs`
  with `<kbd>` chips: `rounded bg-white/5 px-1 py-0.5 text-xs`
- Nav link: `h-8 gap-2 rounded-md px-3 py-1 text-sm text-sidebar-foreground hover:bg-white/5`,
  icon `size-4 shrink-0`
- Section label: `text-xs font-semibold text-sidebar-muted`
- Vertical rhythm via `<div className="h-4 w-full" />` spacers between groups

### Top nav (`TopNav.tsx`)
- `h-16 border-b border-border bg-background px-4 py-3`, `justify-between`
- Vertical divider: `h-4 w-px bg-border`
- Avatar: `size-10 rounded-full bg-accent text-sm font-semibold` (initials)

### Modal (`Modal.tsx`)
- Overlay: `fixed inset-0 z-50 flex items-center justify-center` + `absolute inset-0 bg-background/80` scrim
- Panel: `max-h-[85vh] w-[1313px] max-w-[90vw] overflow-auto border border-border bg-accent`
- Header: `border-b border-border px-6 py-[25px]`, title `text-xl font-semibold`, `X` close (size-4)
- Closes on `Escape` (keydown listener) and scrim click.

### Section header (`SectionHeader.tsx`)
Title (`text-[30px] font-semibold tracking-[-1px]`) + optional badge on the left; a
"View All →" link on the right: `flex items-center gap-1 text-sm font-semibold text-primary hover:opacity-80`
with `ArrowRight` (size-4).

---

## 6. Effects & Motion

### Prism glow (animated marketing accent)
```css
@keyframes prism-glow { 0%,100%{opacity:.85;transform:scale(1)} 50%{opacity:1;transform:scale(1.02)} }
.animate-prism-glow { animation: prism-glow 3s cubic-bezier(0.45,0,0.55,1) infinite; }
```

### Gradient border (glowing top/bottom edges)
```css
.gradient-border { position: relative; border: 1px solid rgba(255,255,255,0.7); }
.gradient-border::before, ::after {
  height: 6px; filter: blur(6px);
  background: linear-gradient(90deg, #3f8dff, #8b5cf6 50%, #3f8dff);   /* blue → violet → blue */
}
```
Used to give cards/heroes a glowing edge. The blue↔violet gradient is the signature accent.

### Shadows
Minimal — only `shadow-xs` / `shadow-sm` on small interactive elements (search, chips, glass
buttons). No large elevation shadows; depth comes from borders + translucent fills.

---

## 7. Layout Conventions

- **Grids** use fractional column ratios pulled straight from Figma, e.g.
  `lg:grid-cols-[806fr_606fr]`, `lg:grid-cols-[465fr_947fr]` — responsive at `lg`.
- **Flex** default: `flex flex-col items-start` for stacked content regions.
- Spacing scale leans on `gap-2 / gap-4 / gap-5 / gap-6`; page padding `px-8 py-10`.
- Uses `svh` units (`min-h-svh`, `100svh`) for mobile-correct viewport heights.
- Absolute pixel widths appear for fixed chrome (sidebar `259px`, modal `1313px`).

---

## 8. Adapting This To Our App (notes)

If porting the *look* (not the React code) to another stack:
1. Replicate the token set: near-black `#0a0a0a` canvas, `#fafafa` text, `#404040` borders,
   near-white `#f5f5f5` primary buttons with black text.
2. Use the **white-over-dark** hover convention (`rgba(255,255,255,0.05→0.10)`) instead of
   solid gray hovers.
3. Keep corners sharp (0–4px), reserve `rounded-full` for pills/avatars only.
4. Geist Sans, semibold headings with negative letter-spacing (`-0.02em` / `~-1px`).
5. Depth via 1px borders + translucent fills, not shadows.
6. Reserve the blue→violet gradient + prism glow strictly for hero/marketing accents.
