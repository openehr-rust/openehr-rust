// The documentation set, read from the vendored Markdown in content/.
//
// Everything here runs at build time only: it is imported from *.server.js
// modules, so neither the Markdown nor the renderer reaches the browser.

import { renderMarkdown, summarize } from './markdown.js';
import { routeFor } from './paths.js';

const raw = import.meta.glob('/content/**/*.md', {
	eager: true,
	query: '?raw',
	import: 'default'
});

/** @type {Record<string, string>} content path -> Markdown source */
const sources = Object.fromEntries(
	Object.entries(raw).map(([key, value]) => [key.replace('/content/', ''), value])
);

const firstHeading = (markdown) => {
	const match = /^#\s+(.+)$/m.exec(markdown);
	return match ? match[1].trim() : '';
};

// Each database crate states how far it has actually been verified, in a
// "## Conformance level: Store" line. That claim is the most important thing
// on the page, so the site lifts it onto the crate list rather than making a
// reader open all six to find out which ones have run against a real server.
const conformanceLevel = (markdown) => {
	const match = /^##\s+Conformance level:\s*(.+)$/m.exec(markdown);
	return match ? match[1].trim() : null;
};

/**
 * The crates, in the order the site lists them: the core Reference Model
 * crate, then the shared persistence engine, then the database crates.
 * The order matches bin/sync-content.mjs.
 */
export const crates = [
	'openehr',
	'openehr-store',
	'openehr-sqlite',
	'openehr-postgresql',
	'openehr-mysql',
	'openehr-mariadb',
	'openehr-mssql',
	'openehr-oracle'
]
	.map((name) => `crates/${name}.md`)
	.filter((file) => file in sources)
	.map((file) => {
		const markdown = sources[file];
		const name = /^crates\/(.+)\.md$/.exec(file)[1];
		return {
			file,
			name,
			route: /** @type {string} */ (routeFor(file)),
			title: firstHeading(markdown) || name,
			summary: summarize(markdown),
			conformance: conformanceLevel(markdown),
			core: name === 'openehr'
		};
	});

/**
 * Specification pages in reading order: the map first, then the numbered
 * chapters, then the two reference tables that are not part of the sequence.
 */
export const specPages = Object.keys(sources)
	.filter((file) => file.startsWith('spec/'))
	.sort((a, b) => {
		const rank = (file) => {
			const name = file.slice('spec/'.length);
			if (name === 'index.md') return 0;
			return /^\d/.test(name) ? 1 : 2;
		};
		return rank(a) - rank(b) || a.localeCompare(b);
	})
	.map((file) => ({
		file,
		route: /** @type {string} */ (routeFor(file)),
		title: firstHeading(sources[file]) || file,
		summary: summarize(sources[file])
	}));

/** Every route this site publishes from content, in prerender order. */
export function routes() {
	return Object.keys(sources)
		.map((file) => ({ file, route: routeFor(file) }))
		.filter((entry) => entry.route !== null);
}

/**
 * Previous/next within a page's own sequence. Crates page through crates and
 * specification pages through the specification; the two never run together,
 * because they are two documents rather than one.
 */
function siblings(route) {
	for (const sequence of [crates, specPages]) {
		const index = sequence.findIndex((entry) => entry.route === route);
		if (index === -1) continue;
		const at = (offset) => {
			const entry = sequence[index + offset];
			return entry ? { title: entry.title, route: entry.route } : null;
		};
		return { previous: at(-1), next: at(1) };
	}
	return { previous: null, next: null };
}

/** Which section a route belongs to, for the breadcrumb. */
function section(file) {
	return file.startsWith('spec/')
		? { label: 'Specification', route: '/spec/' }
		: { label: 'Crates', route: '/crates/' };
}

/**
 * Render one document for a page load.
 *
 * @param {string} route e.g. "/crates/openehr/"
 */
export function document(route) {
	const entry = routes().find((candidate) => candidate.route === route);
	if (!entry) return null;

	const rendered = renderMarkdown(sources[entry.file], { file: entry.file });
	const { previous, next } = siblings(route);

	return {
		route,
		file: entry.file,
		title: rendered.title,
		summary: rendered.summary,
		html: rendered.html,
		headings: rendered.headings,
		section: section(entry.file),
		previous,
		next
	};
}
