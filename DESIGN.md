# Design

ensign's face answers to the same constitution as its source: a closed
vocabulary, declared territories, violations a freshman can point at.
The aesthetic is **clean and clear, systematically laid out** — the
classic Google shape as a base, extended. Light by default, one accent,
generous space, a real type and spacing scale. It is the shared design
language the fleet inherits.

## The shared substrate

Generic tokens, themes, layout, fields, controls, and content come from
`@perish/react-components`; Vite materializes the shared Design runtime
through `@perish/vite-plugin-design`.

Ensign keeps product compositions under `apps/web/src/lib/components`.
They compose shared primitives and semantic markup but own no stylesheet,
style declaration, or parallel token foundation.

## The tokens

| dimension | tokens | shape |
| --- | --- | --- |
| color | `ground` `panel` `well` `rule` `ink` `bright` `muted` `accent` `glow` `warn` `flush` | page, card, sunken input, hairline, body / heading / secondary text, one accent, its tint, the warning ink and its wash |
| type | `fine` `body` `lead` `title` `hero` | fine print, prose, deck, section head, wordmark |
| space | `gap` `step` `room` `span` `rise` | 0.25 / 0.5 / 1 / 1.5 / 2 rem — an 8px grid |
| form | `radius` `bead` `line` `rim` `card` `sheet` | corner, pill, hairline, focus ring, auth-card width, content-page width |
| motion | `beat` | one transition |
| number | `leading` `heft` | line height, medium weight |
| depth | `lift` | one elevation shadow |
| face | `sans` `mono` | the two families |

An unused token is a squatter and gets evicted.

## Territory

Only the shared Design package declares style. Ensign's `lib/components`
territory is style-free, `Shell` loads the shared runtime, and every view
mounts inside it.

## The bar

The face is judged adversarially: a codex + grok cross-review must find
no defect for two consecutive rounds before the UI ships. The review is
the gate.
