// Constants shared by the client-side components. Keep this free of any
// content imports so it stays cheap to ship to the browser.

export const SITE_URL = 'https://openehr-rust.github.io';
export const SITE_NAME = 'openEHR for Rust';
export const ORGANIZATION = 'https://github.com/openehr-rust';

/**
 * Upstream repository. The crate pages link back to the file they render.
 *
 * One monorepo, not the `openehr` / `openehr-databases` split this used to
 * name — those repositories do not exist, and every "view on GitHub" link
 * this produced for a page the site does not publish was dead. Both keys
 * point at the same URL now and are kept separate for the reason `paths.js`'s
 * `blobUrl` gives: absorbing a future real split without touching every call
 * site.
 */
export const REPOSITORIES = {
	core: 'https://github.com/openehr-rust/openehr-rust',
	databases: 'https://github.com/openehr-rust/openehr-rust'
};

/** The organization page is what a footer or a header link should point at. */
export const REPOSITORY = ORGANIZATION;

/** Themes vendored into static/themes/ by bin/sync-lily.mjs. */
export const THEMES = ['light', 'dark', 'nord', 'dracula', 'emerald', 'night'];

export const THEME_LABELS = {
	light: 'Light',
	dark: 'Dark',
	nord: 'Nord',
	dracula: 'Dracula',
	emerald: 'Emerald',
	night: 'Night'
};
