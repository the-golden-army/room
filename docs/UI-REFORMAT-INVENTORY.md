# UI reformat inventory — ROOM

Read-only audit before any change. Framework: Rust/Actix, HTML generated in `.rs` files,
TailwindCSS v4 (`assets/app.css` → `assets/app.dist.css`), vanilla JS (`assets/app.js`).
No React, no component library — every "component" is a `format!` string or a CSS class.

## Routes

| # | Route | File | Data | Loading | Empty | Error | Dark OK |
|---|---|---|---|---|---|---|---|
| 1 | `/` | `templates/chat.rs` | conversations + messages | skeleton in `app.js` | empty state | toast | yes |
| 2 | `/files` | `templates/files.rs` | file tree | skeleton | empty state | toast | yes |
| 3 | `/calls` | `templates/calls.rs` | call history | skeleton | empty state | toast | yes |
| 4 | `/account` | `templates/account.rs` | profile + devices | — | — | toast | yes |
| 5 | `/admin` | `templates/admin.rs` | 8 tabs | — | empty states | toast | yes |
| 6 | `/gate` | `templates/gate.rs` | owner lookup | n/a | n/a | inline error | yes |
| 7 | `/waiting` | `templates/gate.rs` | device row | n/a | n/a | n/a | yes |

## Current design tokens (`assets/app.css`)

```
brand-50…950   violet ramp, 600 = #5333f4
ember-50…600   orange, secondary
surface #f6f6fa  panel #ffffff  panel2 #f3f3f8  panel3 #ececf3
line #e5e5ee  line2 #d6d6e2
ink #17161f  ink2 #55546a  ink3 #8b8a9e
ring #5333f4
dark: surface #0f0e16 … ink #eceaf4   (full dark ramp present)
```

## Audit against the twenty AI tells

| # | Tell | Status |
|---|---|---|
| 1 | Gradient text | pass — none |
| 2 | Radial glow behind hero | pass — none |
| 3 | Emoji as icons | **pass** — inline SVG `icon()` system, 0 emoji verified |
| 4 | Inter/system-ui for everything | **FAIL** — `--font-sans` is `ui-sans-serif, system-ui…`, no display face |
| 5 | Everything centred | minor — 11 `text-center`, mostly empty states (acceptable) |
| 6 | One radius everywhere | **FAIL** — 17 `rounded-full`, 15 `rounded-2xl`, 12 `rounded-xl`, 8 `rounded-lg` with no rule |
| 7 | Three identical feature cards | pass — real lists, not feature cards |
| 8 | Eyebrow pill above headline | pass |
| 9 | Gradient CTA + glow | pass — flat brand buttons |
| 10 | Fake browser/terminal chrome | pass |
| 11 | Trust bar with bold numbers | minor — admin stat cards, but they are real counts |
| 12 | One accent doing six jobs | **FAIL** — brand violet is on nav, buttons, links, focus ring, selection, chips |
| 13 | Monospace as decoration | pass — mono only on timestamps/`kbd` |
| 14 | Testimonial cards | pass |
| 15 | No focal point | minor |
| 16 | Drop shadows on flat containers | **FAIL** — `shadow-soft` on panels that do not float |
| 17 | Every gap the same | minor |
| 18 | Every section same height | n/a |
| 19 | Gradient mesh/blob background | pass |
| 20 | Section headings same size | minor |

**Score: 4 fails → reads as generated on tells 4, 6, 12, 16.**

## Type scale today

`9 / 9.5 / 10 / 10.5 / 11 / 11.5 / 12 / 12.5 / 13 / 13.5 / 15 px` — eleven steps inside a
6px range. That is the "1 : 1.2" problem the skill names: hierarchy is carried by colour and
weight instead of size. No display face at all.

## Dark mode

Present and working: `html.dark` token block, `no-motion` class honours reduced motion,
`:focus-visible` ring, `::selection`. **Keep as-is** — this part is already production-grade.

## Gaps vs `global-ui-reformat`

- Skeletons: exist in `app.js` (verify per-route coverage)
- Human errors: toasts exist; raw server text check needed
- Onboarding: none (no first-run tour)
- `docs/STYLE.md`: does not exist — nothing stops the next change drifting back

## Constraint discovered

`A binary is built in CI; `assets/app.dist.css` is committed and NOT built by CI`, so any
token change must be compiled locally and committed with the source.
