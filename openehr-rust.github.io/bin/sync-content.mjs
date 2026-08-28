#!/usr/bin/env node
// Vendor the crate documentation into content/ so this site builds standalone.
//
// The crates are the source of truth. This site renders them; it never edits
// them. Every file below is copied verbatim, so a page on this site and the
// README a reader sees on GitHub or docs.rs are the same document.
//
// Sources, each overridable so CI can check out the repos anywhere:
//   $OPENEHR            else the sibling checkout ../openehr
//   $OPENEHR_DATABASES  else the sibling checkout ../openehr-databases
//
// Run after either repository changes:  npm run sync

import { cp, mkdir, rm, readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const core = resolve(process.env.OPENEHR ?? join(siteRoot, '..', 'openehr'));
const databases = resolve(process.env.OPENEHR_DATABASES ?? join(siteRoot, '..', 'openehr-databases'));

const missing = [];
if (!existsSync(join(core, 'README.md'))) missing.push(`OPENEHR=${core}`);
if (!existsSync(join(databases, 'openehr-store'))) missing.push(`OPENEHR_DATABASES=${databases}`);
if (missing.length) {
	console.error(`Missing checkout(s). Set: ${missing.join(' ')}`);
	process.exit(1);
}

// The database crates, in the order the site lists them: the shared engine
// first, then the engines by how much of the conformance suite each one has
// actually run. Add a crate here and it appears on the site.
const databaseCrates = [
	'openehr-store',
	'openehr-sqlite',
	'openehr-postgresql',
	'openehr-mysql',
	'openehr-mssql',
	'openehr-oracle'
];

const contentDir = join(siteRoot, 'content');
await rm(contentDir, { recursive: true, force: true });
await mkdir(join(contentDir, 'crates'), { recursive: true });
await mkdir(join(contentDir, 'spec'), { recursive: true });

let count = 0;

// The core crate's README becomes /crates/openehr/.
await cp(join(core, 'README.md'), join(contentDir, 'crates', 'openehr.md'));
count += 1;

for (const crate of databaseCrates) {
	const from = join(databases, crate, 'README.md');
	if (!existsSync(from)) {
		console.warn(`skip (missing): ${crate}/README.md`);
		continue;
	}
	await cp(from, join(contentDir, 'crates', `${crate}.md`));
	count += 1;
}

// openehr-store carries the conformance suite every engine runs, and all five
// engine READMEs link to it. Publishing it keeps those links on the site.
const conformance = join(databases, 'openehr-store', 'spec', 'conformance.md');
if (existsSync(conformance)) {
	await cp(conformance, join(contentDir, 'crates', 'openehr-store-conformance.md'));
	count += 1;
} else {
	console.warn('skip (missing): openehr-store/spec/conformance.md');
}

// The core crate's normative specification becomes /spec/.
const specDir = join(core, 'spec');
for (const entry of (await readdir(specDir)).sort()) {
	if (!entry.endsWith('.md')) continue;
	await cp(join(specDir, entry), join(contentDir, 'spec', entry));
	count += 1;
}

console.log(`Synced ${count} files from ${core} and ${databases}.`);
