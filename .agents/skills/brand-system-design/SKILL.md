---
name: brand-system-design
description: Create or review InterviewScribe logos, icons, visual identity, design tokens, and UI styling while preserving the approved brand system.
---

# InterviewScribe brand system

Read `docs/UI_UX.md` and inspect the current approved logo assets before changing branding or interface styling. A proposal is not an approved asset until the user explicitly validates it.

## Brand idea

InterviewScribe turns spoken exchanges into a clear, structured and trustworthy transcript. The approved mark uses two opposing speech bubbles around three transcript lines. This directly represents multiple speakers becoming one readable exchange.

## Logo rules

- Use `assets/interviewscribe-logo.svg` as the approved master logo.
- Preserve the two opposing speech bubbles and the three centered transcript lines.
- Preserve simple shapes, optical balance and recognition at 20 pixels.
- Use the approved master SVG as the source for every derived icon.
- Keep a clear area of at least one quarter of the mark's width.
- Use solid colors only. Do not add gradients, shadows, bevels, glass, glow or 3D effects.
- Do not add microphones, headphones, sound waves, quotation marks, AI sparkles or decorative letters.
- Do not stretch, rotate, outline, merge or rearrange the approved mark.
- Do not replace the two bubbles with a single bubble or card.
- Keep logo proposals separate from production assets until validation.

## Color system

- Primary blue: `#3156A3`.
- Primary blue hover: `#29498B`.
- Focus ring: `#9AB6EF`.
- Light background: `#F6F7F9`.
- Light surface: `#FFFFFF`.
- Dark background: `#111318`.
- Dark surface: `#191C23`.
- Main light text: `#171A21`.
- Main dark text: `#EFF1F5`.

Speaker colors are functional metadata, not brand colors. Use them on small identifiers and verify contrast independently.

## UI rules

- Make the transcript and recording status the visual priority.
- Use neutral surfaces, thin borders, restrained radii and regular spacing.
- Use one primary action per step.
- Meet WCAG AA and never communicate state through color alone.
- Provide visible keyboard focus and touch targets of at least 44 by 44 pixels.
- Support light and dark themes without changing the brand geometry.
- Avoid decorative animation, oversized headings, excessive cards and artificial dashboard density.

## Asset delivery

For an approved logo, maintain the source SVG and derive PNG, ICO and Android adaptive icons from it. Verify transparent edges, centering, padding and legibility at 16, 20, 32, 48, 128, 256 and 512 pixels before replacing production icons.
