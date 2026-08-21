# UI/UX Typography Resources

Curated resources for typography (font choice, sizing, hierarchy, line height,
spacing, accessibility) tailored for apps.

## Official Platform Guidelines (Start Here)

Most authoritative sources — foundation for a type system.

- **Material Design 3 Typography** (Android / cross-platform)
  m3.material.io/styles/typography
  Clear roles (Display, Headline, Title, Body, Label) with Large/Medium/Small
  sizes. Excellent practical scale and guidance on applying type.

- **Apple Human Interface Guidelines – Typography**
  developer.apple.com/design/human-interface-guidelines/typography
  Recommended sizes (e.g. Body 17 pt on iOS), Dynamic Type support, minimum
  sizes, and hierarchy rules. Essential for Apple platforms.

- **U.S. Web Design System (USWDS) Typography**
  designsystem.digital.gov/components/typography
  Strong accessibility focus, practical font-size tokens, line-length guidance
  (45–90 characters), and line-height recommendations.

## Practical Guides & Best Practices

- **Font Size Guidelines for Responsive Websites** (Learn UI Design)
  learnui.design/blog/mobile-desktop-website-font-size-guidelines.html
  Mobile vs desktop size ranges, input field rules (>=16 px), advice on keeping
  the number of sizes small.

- **Type Scale Cheat Sheet** (UX Encyclopedia)
  ux.detroit3d.com/foundations/type-scale-cheatsheet.html
  Ready-to-use scales (including Material-style), line-height recommendations,
  cross-platform comparisons.

- **Building a Type Scale for Design Systems** (FontFYI)
  fontfyi.com/blog/building-type-scale-design-system
  How to choose ratios (Major Third 1.25 is a common sweet spot), semantic
  tokens, practical implementation tips.

- **Typography Principles for Designers**
  thecrit.co/resources/typography-principles-guide
  Prioritizes hierarchy -> spacing -> contrast -> typeface. Practical order for
  fixing problems.

## Quick Starting Rules (Most Sources Agree)

| Element | Size (Mobile) | Size (Desktop) | Line Height | Notes |
|---|---|---|---|---|
| Body text | 16–18 px | 16–20 px | 1.4–1.6 | Never go below 16 px for body |
| Secondary / Caption | 12–14 px | 13–15 px | 1.3–1.5 | Use lighter weight/color |
| Labels / Buttons | 14–16 px | 14–16 px | ~1.2–1.4 | Medium or SemiBold |
| H3 / Subheading | 18–22 px | 20–24 px | 1.2–1.4 | |
| H2 / Section | 22–28 px | 24–32 px | 1.15–1.3 | |
| H1 / Page Title | 28–36 px | 32–48 px | 1.1–1.25 | |
| Display / Hero | 36–48+ px | 48–72+ px | ~1.1 | Use sparingly |

Other key principles:

- Use as few sizes as possible (4–7 is usually enough).
- Create hierarchy with size + weight + color (at least two of the three).
- Prefer modular scales (1.2 Minor Third or 1.25 Major Third are popular).
- Keep line length ~45–75 characters for readable body text.
- Support accessibility (WCAG contrast, Dynamic Type / user font scaling).
- Test on real devices — optical size matters a lot.

## Tools

- Type Scale generators (search "type scale calculator") — quickly generate
  modular scales.
- Figma / design-tool plugins that implement Material or Apple type scales.
- Google Fonts + variable fonts for flexible weight/optical-size control.
