import { crates } from '$lib/docs.js';

export function load() {
	return {
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
