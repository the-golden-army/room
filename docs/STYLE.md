# Style — ROOM

The brief the next change has to respect. Written once, here, so the design
does not drift back to the average.

```
ANCESTOR         a network switch panel — the physical box a LAN runs through.
                 ROOM's job is literally "who is plugged in, and is the lamp lit".

PALETTE          paper   #fafafa   the page
                 panel   #ffffff   a raised surface (a panel, not a card)
                 panel2  #f2f2f2   sunken: table head, wells, code
                 line    #e2e2e2   hairline
                 ink     #0a0a0a   text
                 ink2    #6e6e6e   secondary text
                 ink3    #9a9a9a   timestamps, meta
                 ACCENT  #0f7a4a   signal green. Used ONLY for state that means
                                   "connected / approved / live" and the single
                                   primary button on a panel.

TYPE             display = ui-monospace 500   (mono IS the display face)
                 body    = system-ui 14px / 1.6
                 label   = mono 11px, uppercase, tracking .12em
SCALE            40 / 24 / 17 / 14 / 11   (ratios 1 : 1.5 : 1.85 : 2.4 : 3.6)
LAYOUT           12-col feel; hairline-separated rows, a left rail for nav,
                 content left-aligned, ragged right. No centred body copy.
RADIUS           3px (`--radius-swx`). Circles only for spinners and ping rings.
SHADOW           none on surfaces. `--shadow-pop` only for things that genuinely
                 float: modal, popover, toast, hover action bar.
SIGNATURE        the status strip (`.statusstrip`) — every peer as a "port" with
                 a lamp, name and ON/OFF, in dense mono.
BANNED           gradients · emoji as icons · radial glows · centred body text ·
                 radius > 4px · shadows on flat panels · accent doing more than
                 ~5% of the surface · pill-shaped chips and badges · fake chrome
```

## Accent discipline

`#0f7a4a` (dark mode `#3ddc97`) is allowed on exactly these:

1. the primary button on a panel (`.btn-brand`)
2. a "live / approved / online" lamp (`.presence.on`, `.port.is-on`, `.chip-brand`)
3. the focus ring

It is **not** allowed on: headings, nav active states, bubbles, borders, tab
labels, avatars, or backgrounds. Nav active is ink. Own message bubbles are ink.

## Measured before / after

| | Before | After |
|---|---|---|
| Type steps in use | 11 (9 → 15px) | 5 (10 / 11 / 12 / 14 / 17 + 24 display) |
| Radius values | 4 arbitrary + pills | 1 (3px) + 2px and 1px for lamps |
| Shadows on flat panels | 7 | 0 |
| Gradient in logo/favicon | yes (violet gradient) | no — flat ink panel |
| Accent colours doing jobs | 1 violet doing 6 | 1 green doing 3 |

## REJECTED

Two directions were offered and not chosen:

- **B — Paper Ledger** (R1 Paper & Ink, shifted to warm cream + brick
  `#A8331B`, Iowan Old Style serif, 2px radius, messages as ledger entries).
  Rejected because ROOM's hardest screens are file tables and admin spec
  lists — serif prose fights dense tabular data, and the warm paper would
  have been the fourth product in a row to use an editorial palette.
- **C — Night Radio** (R2 Swiss inverted: Inter Tight 900, 0px radius, solid
  `4px 4px 0` offset shadows, signal red `#FF2D00`). Rejected because
  brutalist scale suits a landing page, not a chat you keep open all day —
  every message row would have competed for attention.

Chosen: **A — Switchboard**, because the ancestor is what ROOM *is*.

## Rules for the next change

1. New colour? It goes in `@theme` with a one-line job description, or it does
   not go in.
2. New size? Use the five-step scale. If it needs a sixth, justify it here.
3. New surface? `border-radius: var(--radius-swx)`, a hairline, no shadow.
4. Numerals in tables, counts and timestamps get tabular figures (`.tnum`).
5. Every interactive element keeps a visible `:focus-visible` ring.
6. `assets/app.dist.css` is committed and CI does **not** build it — run
   `npx @tailwindcss/cli -i assets/app.css -o assets/app.dist.css --minify`
   and commit the result alongside any class change.
