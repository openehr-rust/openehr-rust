import { crates, specPages } from '$lib/docs.js';
import { SITE_NAME } from '$lib/site.js';

export function load() {
	return {
		// `data.title` is the page.data.title convention every route's load
		// follows: the exact string the page puts in <svelte:head><title>,
		// computed once here so +layout.svelte can read page.data.title for
		// SharePicker without recomputing per-route title logic.
		title: `${SITE_NAME} — openEHR Reference Model crates for Rust`,
		crates: crates.map(({ name, route, title, summary, conformance, core }) => ({
			name,
			route,
			title,
			summary,
			conformance,
			core
		})),
		specCount: specPages.length
	};
}
