#!/usr/bin/env node
// Vendor the Lily Design System pieces this site uses.
//
// Lily's Svelte headless components and helpers are MIT licensed. The helpers
// are not published to npm, and the local checkout runs ahead of the published
// headless package, so the site vendors both from the sibling checkout and
// records the source commit in src/lib/lily/VENDOR.md.
//
// Source: $LILY if set, else ~/git/lilydesignsystem/lily-design-system.
// Run after Lily changes:  npm run sync:lily

import { cp, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const lily = resolve(process.env.LILY ?? join(homedir(), 'git', 'lilydesignsystem', 'lily-design-system'));

const headless = join(lily, 'lily-design-system-svelte-headless', 'components');
const helpers = join(lily, 'lily-design-system-svelte-helpers');
const themes = join(lily, 'themes');

if (!existsSync(headless)) {
	console.error(`No Lily checkout at ${lily}. Set LILY=/path/to/lily-design-system.`);
	process.exit(1);
}

// Only the components the site actually renders. Add to this list rather than
// copying all 492 — every vendored file is code this repo now carries.
const components = [
	'SkipLink',
	'Header',
	'Footer',
	'ArticleLayout',
	'Card',
	'Badge',
	'ContentsNav',
	'ContentsList',
	'ContentsListItem',
	'BreadcrumbNav',
	'BreadcrumbList',
	'BreadcrumbListItem',
	'PaginationNav',
	'PaginationList',
	'PaginationListItem'
];

// Helper packages: directory name -> component file.
const helperComponents = [
	['lily-design-system-svelte-theme-picker', 'ThemePicker.svelte'],
	['lily-design-system-svelte-text-size-picker', 'TextSizePicker.svelte']
];

const lilyDir = join(siteRoot, 'src', 'lib', 'lily');
await rm(lilyDir, { recursive: true, force: true });
await mkdir(join(lilyDir, 'components'), { recursive: true });
await mkdir(join(lilyDir, 'helpers'), { recursive: true });

for (const name of components) {
	const from = join(headless, name, `${name}.svelte`);
	if (!existsSync(from)) {
		console.error(`missing component: ${name}`);
		process.exit(1);
	}
	await cp(from, join(lilyDir, 'components', `${name}.svelte`));
}

for (const [pkg, file] of helperComponents) {
	const from = join(helpers, pkg, file);
	if (!existsSync(from)) {
		console.error(`missing helper: ${pkg}/${file}`);
		process.exit(1);
	}
	await cp(from, join(lilyDir, 'helpers', file));
}

// Themes. This is a documentation site for Rust developers rather than a
// deployment inside one health service, so the set is light and dark plus a
// few well-known developer palettes — not the national health service themes,
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
	join(lilyDir, 'VENDOR.md'),
	`# Vendored Lily Design System

These files are copied verbatim from the Lily Design System (MIT licence) by
\`bin/sync-lily.mjs\`. Do not edit them here — change them upstream and re-run
\`npm run sync:lily\`.

- Source: <https://github.com/LilyDesignSystem>
- Commit: \`${commit}\`
- Components: ${components.join(', ')}
- Helpers: ${helperComponents.map(([, file]) => file.replace('.svelte', '')).join(', ')}
- Themes: \`static/themes/\` (${themeFiles.length} files)
`
);

console.log(
	`Vendored ${components.length} components, ${helperComponents.length} helpers, ${themeFiles.length} themes from ${lily} (${commit.slice(0, 9)}).`
);
