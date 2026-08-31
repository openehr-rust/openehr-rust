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

`bin/sync-lily.mjs` vendors the Lily components, helpers, and themes the site
uses, and records the source commit in `src/lib/lily/VENDOR.md`. Those files
are copies: change them upstream in Lily, then re-run the sync.

```sh
npm run sync:lily                        # from ~/git/lilydesignsystem/lily-design-system
LILY=/path/to/lily-design-system npm run sync:lily
```

Six themes ship in `static/themes/`: light, dark, Nord, Dracula, Emerald, and
Night. The theme and text-size pickers persist to `localStorage` and the theme
follows the system preference until a reader chooses one.

## Develop

```sh
npm install
npm run dev        # http://localhost:5173
npm run build      # prerender to build/
npm run preview    # serve build/ as GitHub Pages will
```

## Deploy

**This directory is not what GitHub Pages serves.** It's a subdirectory of the
`openehr-rust/openehr-rust` monorepo, and GitHub Actions only discovers
`.github/workflows/` at a repository's own root — a workflow nested under a
monorepo subdirectory never runs, whatever it says. An organization site also
has to be served from a repository literally named `<org>.github.io`, which
this monorepo isn't.

The actual publish target is the separate sibling repository
[`openehr-rust/openehr-rust.github.io`](https://github.com/openehr-rust/openehr-rust.github.io),
holding a rewritten export of just this subdirectory's history — see
[`spec/monorepo-github-pages/`](../spec/monorepo-github-pages/index.md).
**Edit here, as always; never commit directly to that sibling repo**, or the
next export silently discards it. To publish:

```sh
# from the monorepo root
scripts/publish-pages-subtree.py            # split + build-verify, no push
scripts/publish-pages-subtree.py --push     # split + build-verify + force-push
```

This `.github/workflows/deploy.yml` is what the *exported* copy runs, at its
new root, on every push to the sibling's `main`: it builds and publishes
`build/` to GitHub Pages. In that repository's settings, **Pages → Build and
deployment → Source** must be set to **GitHub Actions**.

Two details make GitHub Pages work there, and both are load-bearing:

- `static/.nojekyll` stops GitHub running Jekyll over the output, which would
  otherwise drop the `_app/` directory.
- `paths.base` stays empty because the repository is named after the
  organization and is served from the root. A project-pages repository would
  need `paths: { base: process.env.BASE_PATH }` instead.

## Licence

The site code is MIT OR Apache-2.0, matching the crates. The vendored Lily
files are MIT, from the Lily Design System.

openEHR specifications are published by the
[openEHR Foundation](https://openehr.org/) under CC-BY-SA. These crates are an
independent implementation and are not endorsed by or affiliated with the
openEHR Foundation.
