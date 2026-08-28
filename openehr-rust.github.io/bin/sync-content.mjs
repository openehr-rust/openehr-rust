#!/usr/bin/env node
// Vendor the crate documentation into content/ so this site builds standalone.
//
// The crates are the source of truth. This site renders them; it never edits
// them. Every file below is copied verbatim, so a page on this site and the
// README a reader sees on GitHub or docs.rs are the same document.
//
// Source, overridable so CI can check out the repo anywhere:
//   $OPENEHR_RUST  else the sibling checkout ../..
//
// `openehr-rust` is one monorepo, not the two-repo split (`openehr` plus a
// separate `openehr-databases`) this script originally assumed -- every
// crate below is a sibling directory at the repository root, this site
// included, one level down (`openehr-rust.github.io/`). That assumption came
// from elsewhere and never matched this repository: it named six database
// crates and left out `openehr-mariadb`, a real, published crate, and every
// path it read pointed at directories (`openehr-databases/...`) that do not
// exist here. `spec/databases/conformance-matrix.md` calls `openehr-sqlite`
// **Verified**; the content this produced before the fix still said
// **Store**, dated 2026-08-01 -- vendored from somewhere else's snapshot,
// never actually generated from this repository's files. Caught by
// `scripts/check-docs.py` in the parent monorepo, which reads every `*.md`
// under here same as anywhere else and does not know this directory is
// "vendored" rather than authored.
//
// Run after any crate's docs change:  npm run sync

import { cp, mkdir, rm, readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const root = resolve(process.env.OPENEHR_RUST ?? join(siteRoot, '..'));

const missing = [];
if (!existsSync(join(root, 'openehr', 'README.md'))) missing.push(`OPENEHR_RUST=${root}`);
if (missing.length) {
	console.error(`Missing checkout. Set: ${missing.join(' ')}`);
	process.exit(1);
}

// The database crates, in the order the site lists them: the shared engine
// first, then the engines by how much of the conformance suite each one has
// actually run (`spec/databases/conformance-matrix.md`'s own ladder, not this
// script's opinion). Six real crates -- `openehr-mariadb` was missing here
// and appeared nowhere on the site as a result. Add a crate here and it
// appears on the site; the ordering is the one thing this list still decides
// by hand, because the level itself is read from each README, not asserted.
const databaseCrates = [
	'openehr-store',
	'openehr-sqlite',
	'openehr-postgresql',
	'openehr-mysql',
	'openehr-mariadb',
	'openehr-mssql',
	'openehr-oracle'
];

const contentDir = join(siteRoot, 'content');
await rm(contentDir, { recursive: true, force: true });
await mkdir(join(contentDir, 'crates'), { recursive: true });
await mkdir(join(contentDir, 'spec'), { recursive: true });

let count = 0;

// The core crate's README becomes /crates/openehr/.
await cp(join(root, 'openehr', 'README.md'), join(contentDir, 'crates', 'openehr.md'));
count += 1;

for (const crate of databaseCrates) {
	const from = join(root, crate, 'README.md');
	if (!existsSync(from)) {
		console.warn(`skip (missing): ${crate}/README.md`);
		continue;
	}
	await cp(from, join(contentDir, 'crates', `${crate}.md`));
	count += 1;
}

// openehr-store carries the conformance suite every engine runs, and all six
// engine READMEs link to it. Publishing it keeps those links on the site.
const conformance = join(root, 'openehr-store', 'spec', 'conformance.md');
if (existsSync(conformance)) {
	await cp(conformance, join(contentDir, 'crates', 'openehr-store-conformance.md'));
	count += 1;
} else {
	console.warn('skip (missing): openehr-store/spec/conformance.md');
}

// The core crate's normative specification becomes /spec/.
const specDir = join(root, 'openehr', 'spec');
for (const entry of (await readdir(specDir)).sort()) {
	if (!entry.endsWith('.md')) continue;
	await cp(join(specDir, entry), join(contentDir, 'spec', entry));
	count += 1;
}

console.log(`Synced ${count} files from ${root}.`);
