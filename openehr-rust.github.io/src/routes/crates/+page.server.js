import { crates } from '$lib/docs.js';
import { SITE_NAME } from '$lib/site.js';

export function load() {
	return {
		// See +page.server.js at the site root for the page.data.title convention.
		title: `Crates — ${SITE_NAME}`,
		crates: crates.map(({ name, route, title, summary, conformance, core }) => ({
			name,
			route,
			title,
			summary,
			conformance,
			core
		}))
	};
}
