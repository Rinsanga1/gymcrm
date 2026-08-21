# Dashboard Design Resources

Resources for designing effective analytics dashboards.

## Foundational Books & Expert Guidance

- **Stephen Few – Information Dashboard Design** (the classic reference)
  Covers the 13 most common design mistakes, visual perception principles,
  single-screen layout, proper use of color, matching display media to data.
  Free articles on his site (Perceptual Edge) are also excellent.
- **Edward Tufte principles** (Data-Ink Ratio, eliminate chartjunk)
  Maximize the pixels that represent data; remove decorative elements.

## Practical Best-Practice Guides

- **10 Dashboard Design Principles and Best Practices** (TechTarget) — prioritize
  simplicity, provide context, limit KPIs, think about UX.
- **Analytics Dashboard Design: Best Practices That Drive Action** (Designpixil)
  — metric hierarchy, chart selection, filtering, drill-downs, empty states.
- **Dashboard Design Best Practices: 9 Rules** (Zoho Analytics) — three
  horizontal zones (headline KPIs -> context/trends -> detail), visual hierarchy.
- **10 Dashboard Design Rules, 6 Years Later** (Valiotti Data) — purpose
  definition, chart limits, layout, pre-ship checklist.
- **SaaS Analytics Dashboard UX Patterns** (SaaS UI) — KPI rows, progressive
  disclosure, comparisons, scoping controls.

## Design Systems & Component Guidance

- **Ant Design – Visualization / Data Display** — summary first -> filters ->
  details, chart selection, layout for data-heavy interfaces.
- **Material Design / MUI dashboard templates, Ant Design Pro** — ready-made
  KPI cards, charts, tables, filters.

## Layout & Hierarchy (most sources converge on this)

- **Top zone (above the fold)** — 3–5 primary KPIs. Large numbers + trend
  arrows/sparklines + comparison (vs previous period or target). Most important
  metric top-left (F-pattern).
- **Middle zone** — supporting context. Trend charts (line for time series),
  breakdowns, comparisons that explain the KPIs.
- **Bottom zone** — detail & exploration. Tables, filters, drill-downs,
  secondary charts. Use progressive disclosure.

Key rules:

- Limit to 5–7 main data elements per view.
- Always pair numbers with context (trend, target, previous period).
- Match chart type to the question (line = trends, bar = comparisons, avoid pie
  charts with >4–5 slices).
- Use color sparingly and semantically (one accent for "what matters",
  red/green only for status).
- Design for the 5-second test: grasp overall status almost immediately.
- Support dashboard types: operational (real-time action), analytical
  (exploration), strategic (high-level overview).

## Inspiration & Real Examples

- Real SaaS products: Stripe, Linear, PostHog, Mixpanel, Amplitude, Vercel,
  Mercury (finance), HubSpot.
- Dashboard collections on Muzli, Dribbble, Behance ("analytics dashboard" /
  "SaaS dashboard").
- Tableau Public and Power BI community galleries for complex examples.

## Quick Chart Selection Cheat Sheet

| Question type | Best chart |
|---|---|
| Trend over time | Line / Area |
| Compare categories | Bar / Column |
| Part-to-whole (few items) | Donut / Stacked bar |
| Distribution / correlation | Scatter / Histogram |
| Exact values / lookup | Table |
| Single KPI + status | Big number + sparkline / bullet |
