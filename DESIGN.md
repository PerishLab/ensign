# Design

ensign's face answers to the same constitution as its source: a closed
vocabulary, declared territories, violations a freshman can point at.
The aesthetic is **clean and clear, systematically laid out** — the
classic Google shape as a base, extended. Light by default, one accent,
generous space, a real type and spacing scale. It is the shared design
language the fleet inherits.

## The four organs

All four live in `packages/components/src`, the sole style territory.

- `tokens.scss` — **defines** the vocabulary. One typed `@property` per
  token; a var not registered here does not exist. Canary initials
  (`magenta`, `0px`) make a theme that forgets a binding confess on
  screen.
- `themes/<theme>.scss` — **binds values**. `light` is the default
  `:root`; `dark` overrides under `prefers-color-scheme: dark`. The only
  files where a design literal (hex, rem, ms) may appear.
- `media.scss` — **defines the seams**: `wide`/`narrow` at 40rem, `dark`,
  `calm`, spelled once each as a mixin; the `--seam` marker lets runtime
  ask which world it is in without learning the numbers.
- `<Atom>.scss` — **consumes**. Sheets speak `var()` plus the enum
  whitelist; seams only through the media mixins. No literals, no raw
  `@media`, ever.

## The tokens

| dimension | tokens | shape |
| --- | --- | --- |
| color | `ground` `panel` `well` `rule` `ink` `bright` `muted` `accent` `glow` | page, card, sunken input, hairline, body / heading / secondary text, one accent, its tint |
| type | `fine` `body` `lead` `title` `hero` | fine print, prose, deck, section head, wordmark |
| space | `gap` `step` `room` `span` `rise` | 0.25 / 0.5 / 1 / 1.5 / 2 rem — an 8px grid |
| form | `radius` `bead` `line` `rim` `card` | corner, pill, hairline, focus ring, auth-card width |
| motion | `beat` | one transition |
| number | `leading` `heft` | line height, medium weight |
| depth | `lift` | one elevation shadow |
| face | `sans` `mono` | the two families |

An unused token is a squatter and gets evicted.

## Territory

Only `packages/components` declares style; `apps/web` consumes
classNames and declares nothing. `Frame` is the shell: it loads tokens +
themes and sets the page. Every view mounts inside it.

## The bar

The face is judged adversarially: a codex + grok cross-review must find
no defect for two consecutive rounds before the UI ships. The review is
the gate.
