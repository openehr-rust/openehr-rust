// Constants shared by the client-side components. Keep this free of any
// content imports so it stays cheap to ship to the browser.

export const SITE_URL = 'https://openehr-rust.github.io';
export const SITE_NAME = 'openEHR for Rust';
export const ORGANIZATION = 'https://github.com/openehr-rust';

/** Upstream repositories. The crate pages link back to the file they render. */
export const REPOSITORIES = {
	core: 'https://github.com/openehr-rust/openehr',
	databases: 'https://github.com/openehr-rust/openehr-databases'
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
