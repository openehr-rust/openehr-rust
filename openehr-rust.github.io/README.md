# openehr-rust.github.io

The website for [openehr-rust](https://github.com/openehr-rust) —
<https://openehr-rust.github.io>.

Built with [SvelteKit](https://svelte.dev/docs/kit) and the
[Lily Design System](https://github.com/LilyDesignSystem), prerendered to
static files by `@sveltejs/adapter-static`, and published by GitHub Pages.

## What this repository is, and is not

It is a **renderer**. The crates are the source of truth: every page on the
site is a Markdown file copied verbatim from
[openehr-rust](https://github.com/openehr-rust/openehr-rust), the one
monorepo that holds all eighteen crates, so a page here and the README a
reader sees on GitHub are the same document. No page content is written in
this repository.

That means a documentation fix belongs upstream, in the crate — not here. Edit
the crate, then re-run `npm run sync`.

## Content

`bin/sync-content.mjs` vendors Markdown into `content/`, which is committed so
the site builds standalone in CI without checking out the crate repository.

| Site | Source |
| --- | --- |
| `/crates/openehr/` | `openehr/README.md` |
| `/crates/<engine>/` | `<engine>/README.md` |
| `/crates/openehr-store/conformance/` | `openehr-store/spec/conformance.md` |
| `/spec/`, `/spec/<page>/` | `openehr/spec/*.md` |

Every path above is relative to the monorepo root: `openehr`, `openehr-store`,
and the six `openehr-<engine>` crates are sibling directories there, this site
included, one level down. (An earlier version of this file assumed a
two-repository split — a standalone `openehr` plus a separate
`openehr-databases` monorepo for everything else — that never existed here;
neither repository does. `bin/sync-content.mjs`'s own header has the
consequence that had, in the code it was found against.)

The overview at `/` and the crate list at `/crates/` are the two pages written
here rather than synced, because no upstream file corresponds to them.

```sh
npm run sync        # re-vendor crate Markdown into content/
```

The source defaults to the sibling checkout `../..` — this site's own parent
directory, i.e. the monorepo root — and is overridable:

```sh
OPENEHR_RUST=/path/to/openehr-rust npm run sync
```

`llms.txt`/`llms.json` (the monorepo root's own, per
`spec/llms-json-and-llms-txt/`) get the same treatment, one script further:
`bin/sync-llms.mjs` reads them, rewrites each entry that this site actually
publishes a page for into a site URL via `src/lib/paths.js`'s `routeFor` —
the same translation the vendored Markdown's own links already go through —
and leaves everything else (most of the repository: root documents,
`agents/`, the skill folders, anything under `spec/` this site does not
render) pointing at GitHub, same as `paths.js` does for those.

```sh
npm run sync:llms   # re-vendor llms.txt/llms.json into static/, rewritten for this domain
```

Links inside the vendored Markdown are written for the crate directory
layouts, which this site flattens. `src/lib/paths.js` translates them: a link
to a page the site publishes becomes a site route, and a link to anything else
— a licence, an example, a spec file not carried here — becomes a link to that
file on GitHub. A relative path is never left to resolve against a site URL,
where it would mean nothing.

## Design system

The site, header, and article chrome (`Header`, `Footer`, `ArticleLayout`,
`Card`, `Badge`, the breadcrumb/contents/pagination nav families, `SkipLink`)
come from
[`lily-design-system-svelte-headless`](https://www.npmjs.com/package/lily-design-system-svelte-headless)
on npm — headless, unstyled Svelte 5 components. The header's three tools —
theme, text size, and share — are each their own package:
[`lily-design-system-svelte-theme-picker`](https://www.npmjs.com/package/lily-design-system-svelte-theme-picker),
[`lily-design-system-svelte-text-size-picker`](https://www.npmjs.com/package/lily-design-system-svelte-text-size-picker),
and
[`lily-design-system-svelte-share-picker`](https://www.npmjs.com/package/lily-design-system-svelte-share-picker).
The share picker is wired with four `targets` — LinkedIn, Mastodon, Bluesky,
Reddit — each an `href` builder in `+layout.svelte`'s `shareTargets`, per the
package's own stance that which networks to offer is an editorial call it
won't make for you; it ships no endpoints. Mastodon has no single share
endpoint (it's federated), so, per the package's own quick-start example,
that builder targets `mastodon.social` specifically — a reader elsewhere
gets a working compose dialog there and re-pastes it into their own instance.
Where the platform has a native share sheet, `strategy="auto"` (the default)
opens that instead of this list. Bump them the ordinary way:

```sh
pnpm update lily-design-system-svelte-headless lily-design-system-svelte-theme-picker \
  lily-design-system-svelte-text-size-picker lily-design-system-svelte-share-picker
```

**The `page.data.title` convention:** every route's `load` returns a `title`
string — the exact text that route's `<svelte:head><title>` renders — so
`+layout.svelte` can read `page.data.title` once, for `SharePicker`, instead
of re-deriving each route's title. Adding a page means adding this field to
its `load`, the same way every existing route does; `+error.svelte` is the
one exception, since an error boundary has no `load` of its own — the
layout falls back to `SITE_NAME` there.

Theme CSS is the one piece that stays vendored rather than a dependency:
each file in `static/themes/` inlines the component CSS for all of Lily's
components into one standalone stylesheet, which is a separate Lily build
target with no npm package of its own — by design, since the headless
components ship no CSS at all. `bin/sync-themes.mjs` copies the six bundles
this site uses from a local Lily checkout and records the source commit in
`static/themes/VENDOR.md`.

```sh
npm run sync:themes                        # from ~/git/lilydesignsystem/lily-design-system
LILY=/path/to/lily-design-system npm run sync:themes
```

Six themes ship in `static/themes/`: light, dark, Nord, Dracula, Emerald, and
Night. The theme and text-size pickers persist to `localStorage` and the theme
follows the system preference until a reader chooses one.

**Theme switching is attribute-based, not link-swapped:** `+layout.svelte`
preloads all six stylesheets (per ThemePicker's own "Preloading for
zero-flicker switching" doc), so every theme's CSS — each scoped to
`:root[data-theme="<slug>"]` — is already present and a switch is just the
`data-theme` attribute changing on `<html>`, with no fetch and no flash of
unstyled content while the new theme loads. `src/app.html` still carries the
one managed `<link data-lily-theme-picker="theme">` ThemePicker itself
swaps; with the matching preload already loaded, that swap is a same-URL
cache hit rather than a real fetch. The cost is real and stated rather than
left for someone to find in a network tab: six stylesheets load upfront
(~145 kB gzipped total) instead of one (~24 kB), because each inlines the
CSS for every Lily component that theme covers.

## Develop

```sh
npm install
npm run dev        # http://localhost:5173
npm run build      # prerender to build/
npm run preview    # serve build/ as GitHub Pages will
```

## Deploy

`.github/workflows/deploy.yml` builds on every push to `main` and publishes
`build/` to GitHub Pages. In the repository settings, **Pages → Build and
deployment → Source** must be set to **GitHub Actions**.

Two details make GitHub Pages work, and both are load-bearing:

- `static/.nojekyll` stops GitHub running Jekyll over the output, which would
  otherwise drop the `_app/` directory.
- `paths.base` stays empty because the repository is named after the
  organization and is served from the root. A project-pages repository would
  need `paths: { base: process.env.BASE_PATH }` instead.

## Licence

The site code is MIT OR Apache-2.0, matching the crates. The Lily Design
System packages are `MIT OR Apache-2.0 OR GPL-2.0-only OR GPL-3.0-only OR
BSD-3-Clause`; the vendored theme stylesheets in `static/themes/` (see
"Design system" above) are MIT, from the Lily Design System.

openEHR specifications are published by the
[openEHR Foundation](https://openehr.org/) under CC-BY-SA. These crates are an
independent implementation and are not endorsed by or affiliated with the
openEHR Foundation.
