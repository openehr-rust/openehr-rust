// Mapping between vendored content files and site routes.
//
// Content paths are always relative to content/, e.g. "crates/openehr.md".
// The vendored files are copied verbatim from the openehr-rust monorepo
// (openehr-rust/openehr-rust), so their links are written for that
// repository's directory layout -- every crate a sibling directory at its
// root, this site included, one level down -- not for this site's flattened
// URL space. Everything here exists to translate the former into the latter,
// and to send anything this site does not publish back to GitHub rather than
// leaving a dead relative link in the rendered page.
//
// This module used to assume a two-repository split (a standalone `openehr`
// repo plus a separate `openehr-databases` monorepo for everything else),
// inherited from elsewhere and never true of this repository. Two defects
// followed from that: `DATABASE_CRATES` omitted `openehr-mariadb`, a real,
// published crate, so it never got a page; and every "view on GitHub" link
// for something this site does not publish resolved against a repository
// that does not exist (`site.js`'s `REPOSITORIES.databases`), which is a
// dead link on a live page rather than something `scripts/check-docs.py` in
// the parent monorepo could ever catch on its own.

import { REPOSITORIES } from './site.js';

/** The six other crates in the same conformance ladder as openehr-store. */
const DATABASE_CRATES = new Set([
	'openehr-store',
	'openehr-sqlite',
	'openehr-postgresql',
	'openehr-mysql',
	'openehr-mariadb',
	'openehr-mssql',
	'openehr-oracle'
]);

/** Normalize a relative or absolute href against the file that contains it. */
export function contentPath(href, fromFile) {
	const from = fromFile.includes('/') ? fromFile.slice(0, fromFile.lastIndexOf('/')) : '';
	const segments = href.startsWith('/')
		? href.slice(1).split('/')
		: [...from.split('/'), ...href.split('/')];
	const out = [];
	for (const segment of segments) {
		if (segment === '' || segment === '.') continue;
		if (segment === '..') out.pop();
		else out.push(segment);
	}
	return out.join('/');
}

/** Site route for a content path, or null when the file is not published. */
export function routeFor(path) {
	if (path === 'crates/openehr-store-conformance.md') return '/crates/openehr-store/conformance/';
	if (path === 'spec/index.md') return '/spec/';
	const crate = /^crates\/([\w.-]+)\.md$/.exec(path);
	if (crate) return `/crates/${crate[1]}/`;
	const spec = /^spec\/([\w.-]+)\.md$/.exec(path);
	if (spec) return `/spec/${spec[1]}/`;
	return null;
}

/** The crate a content file belongs to, or null for specification pages. */
function crateOf(file) {
	if (file === 'crates/openehr-store-conformance.md') return 'openehr-store';
	const match = /^crates\/([\w.-]+)\.md$/.exec(file);
	return match ? match[1] : null;
}

/**
 * A blob URL for a path inside the repository that owns `crate`.
 *
 * `REPOSITORIES.core` and `.databases` are the same URL today -- one
 * monorepo, not two -- kept as separate keys because a future split back into
 * multiple repositories is exactly the kind of change this indirection exists
 * to absorb without touching every call site again.
 */
function blobUrl(crate, path) {
	if (crate === null || crate === 'openehr') {
		return `${REPOSITORIES.core}/blob/main/${path}`;
	}
	return `${REPOSITORIES.databases}/blob/main/${path}`;
}

/**
 * Where a link inside a vendored file points, expressed as a path in the real
 * repository (openehr-rust/openehr-rust) that owns it -- not this site's
 * flattened content/ layout. Every crate is a sibling directory at that
 * repository's root, so this needs no per-crate directory translation; it
 * only has to know which fake filename each crate's content was vendored
 * under, so that resolving a relative link against it lands in the right
 * directory (`contentPath` only ever looks at the directory part).
 */
function repositoryPath(href, fromFile) {
	const crate = crateOf(fromFile);
	if (crate === null) return contentPath(href, `openehr/${fromFile}`); // openehr/spec/<file>.md
	if (crate === 'openehr') return contentPath(href, 'openehr/README.md');
	const base = fromFile.endsWith('-conformance.md') ? `${crate}/spec/x.md` : `${crate}/README.md`;
	return contentPath(href, base);
}

/**
 * Resolve a link written inside a vendored file to a content path this site
 * publishes, or null. This is the step that undoes the flattening: a link from
 * the core README to `spec/01-scope.md` is a site page, and a link from a
 * database crate to `../openehr-store` is the sibling crate's page.
 */
function publishedContentPath(href, fromFile) {
	const crate = crateOf(fromFile);

	// Specification pages link to their siblings by bare filename.
	if (crate === null) return contentPath(href, fromFile);

	const path = repositoryPath(href, fromFile);

	if (crate === 'openehr') {
		// Only the core crate's spec/ is published as /spec/. `repositoryPath`
		// returns the real repo path (`openehr/spec/...`); translate back to
		// this site's flattened content path (`spec/...`) before returning it,
		// since a caller checks this against `routeFor`, which speaks site
		// paths, not repository ones.
		return path.startsWith('openehr/spec/') ? path.slice('openehr/'.length) : null;
	}

	// openehr-store's conformance suite is published under its crate page.
	if (path === 'openehr-store/spec/conformance.md') return 'crates/openehr-store-conformance.md';

	// A bare crate directory, e.g. "../openehr-store", is that crate's page.
	if (DATABASE_CRATES.has(path)) return `crates/${path}.md`;
	const readme = /^([\w.-]+)\/README\.md$/.exec(path);
	if (readme && DATABASE_CRATES.has(readme[1])) return `crates/${readme[1]}.md`;

	return null;
}

/**
 * Rewrite a Markdown link into a site link.
 *
 * External links are left untouched. An internal link to a page this site
 * publishes becomes a site route. An internal link to anything else — a
 * licence file, an example, a spec this site does not carry — becomes a link
 * to that file on GitHub, because a relative path that meant something in a
 * crate directory means nothing at a site URL.
 */
export function rewriteHref(href, fromFile) {
	if (!href) return href;
	if (/^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith('//') || href.startsWith('#')) return href;

	const hashAt = href.indexOf('#');
	const hash = hashAt === -1 ? '' : href.slice(hashAt);
	const target = hashAt === -1 ? href : href.slice(0, hashAt);
	if (!target) return href;

	const published = publishedContentPath(target, fromFile);
	const route = published && routeFor(published);
	if (route) return route + hash;

	return blobUrl(crateOf(fromFile), repositoryPath(target, fromFile)) + hash;
}
