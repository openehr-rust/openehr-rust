#!/usr/bin/env node
// Vendor the Lily Design System theme bundles this site serves.
//
// Lily's Svelte components ship on npm (see package.json:
// lily-design-system-svelte-headless, -theme-picker, -text-size-picker) --
// headless and unstyled, by design. Theme CSS is deliberately not part of
// that distribution: each theme bundle inlines the component CSS for all 492
// Lily components into one standalone stylesheet, which is a different Lily
// build target with no npm package of its own. ThemePicker's own docs put it
// plainly: "Authors drop theme CSS files ... into a directory served by your
// app." So the six files in static/themes/ stay a vendored, hand-refreshed
// snapshot rather than a dependency -- this script is what refreshes them.
//
// Source: $LILY if set, else ~/git/lilydesignsystem/lily-design-system.
// Run after Lily's theme bundles change:  npm run sync:themes

import { cp, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const lily = resolve(process.env.LILY ?? join(homedir(), 'git', 'lilydesignsystem', 'lily-design-system'));

const themes = join(lily, 'themes');

if (!existsSync(themes)) {
	console.error(`No Lily checkout at ${lily}. Set LILY=/path/to/lily-design-system.`);
	process.exit(1);
}

// This is a documentation site for Rust developers rather than a deployment
// inside one health service, so the set is light and dark plus a few
// well-known developer palettes -- not the national health service themes,
// which belong to sites that serve those organizations.
const wantedThemes = ['light', 'dark', 'nord', 'dracula', 'emerald', 'night'];
const themeDir = join(siteRoot, 'static', 'themes');
await rm(themeDir, { recursive: true, force: true });
await mkdir(themeDir, { recursive: true });
const available = new Set(await readdir(themes));
const themeFiles = [];
for (const name of wantedThemes) {
	const file = `${name}.css`;
	if (!available.has(file)) {
		console.error(`missing theme: ${file}`);
		process.exit(1);
	}
	await cp(join(themes, file), join(themeDir, file));
	themeFiles.push(file);
}

let commit = 'unknown';
try {
	commit = execFileSync('git', ['-C', lily, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
} catch {
	// A tarball checkout has no git metadata; provenance is best-effort.
}

await writeFile(
	join(themeDir, 'VENDOR.md'),
	`# Vendored Lily Design System themes

These stylesheets are copied verbatim from the Lily Design System (MIT
licence) by \`bin/sync-themes.mjs\`. Do not edit them here -- change them
upstream and re-run \`npm run sync:themes\`. Unlike the Svelte components,
these bundles are not published to npm: see the script's header comment for
why.

- Source: <https://github.com/LilyDesignSystem>
- Commit: \`${commit}\`
- Themes: ${themeFiles.join(', ')}
`
);

console.log(`Vendored ${themeFiles.length} themes from ${lily} (${commit.slice(0, 9)}).`);
