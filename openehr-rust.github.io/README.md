# openehr-rust.github.io

The website for [openehr-rust](https://github.com/openehr-rust) —
<https://openehr-rust.github.io>.

Built with [SvelteKit](https://svelte.dev/docs/kit) and the
[Lily Design System](https://github.com/LilyDesignSystem), prerendered to
static files by `@sveltejs/adapter-static`, and published by GitHub Pages.

## What this repository is, and is not

It is a **renderer**. The crates are the source of truth: every page on the
site is a Markdown file copied verbatim from one of the crate repositories, so
a page here and the README a reader sees on GitHub are the same document. No
page content is written in this repository.

That means a documentation fix belongs upstream, in the crate — not here. Edit
the crate, then re-run `npm run sync`.

## Content

`bin/sync-content.mjs` vendors Markdown into `content/`, which is committed so
the site builds standalone in CI without checking out the crate repositories.

| Site | Source |
| --- | --- |
| `/crates/openehr/` | `openehr/README.md` |
| `/crates/<engine>/` | `openehr-databases/<engine>/README.md` |
| `/crates/openehr-store/conformance/` | `openehr-databases/openehr-store/spec/conformance.md` |
| `/spec/`, `/spec/<page>/` | `openehr/spec/*.md` |

The overview at `/` and the crate list at `/crates/` are the two pages written
here rather than synced, because no upstream file corresponds to them.

```sh
npm run sync        # re-vendor crate Markdown into content/
```

Sources default to the sibling checkouts `../openehr` and
`../openehr-databases`, and are overridable:

```sh
OPENEHR=/path/to/openehr OPENEHR_DATABASES=/path/to/openehr-databases npm run sync
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

The site code is MIT OR Apache-2.0, matching the crates. The vendored Lily
files are MIT, from the Lily Design System.

openEHR specifications are published by the
[openEHR Foundation](https://openehr.org/) under CC-BY-SA. These crates are an
independent implementation and are not endorsed by or affiliated with the
openEHR Foundation.
