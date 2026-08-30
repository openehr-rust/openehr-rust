<script>
	import { Card, Badge } from 'lily-design-system-svelte-headless';
	import { REPOSITORIES, SITE_NAME, SITE_URL } from '$lib/site.js';

	let { data } = $props();

	const description =
		'openEHR Reference Model types, validation, paths, AQL parsing, change-control ' +
		'security primitives, and persistence — in Rust.';

	const core = $derived(data.crates.find((crate) => crate.core));
	const persistence = $derived(data.crates.filter((crate) => !crate.core));

	// How far the crate has been verified, mapped onto Lily's badge variants.
	// A dialect-only crate is a warning because nothing of it has been run.
	const badgeType = (level) =>
		({ store: 'success', schema: 'info', dialect: 'warning' })[level?.toLowerCase()] ?? 'default';
</script>

<svelte:head>
	<title>{SITE_NAME} — openEHR Reference Model crates for Rust</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={`${SITE_URL}/`} />
	<meta property="og:title" content={SITE_NAME} />
	<meta property="og:description" content={description} />
	<meta property="og:type" content="website" />
	<meta property="og:url" content={`${SITE_URL}/`} />
</svelte:head>

<section class="hero">
	<h1 class="hero-headline">openEHR, in Rust</h1>
	<p class="hero-lede">{description}</p>
	<p class="hero-body">
		<a href="https://specifications.openehr.org/">openEHR</a> specifies clinical information as a
		small, stable Reference Model of about ninety classes, plus archetypes that constrain it into
		clinical content. These crates implement the Reference Model and the machinery around it, so a
		Rust program can read, build, check, address, and safely disclose openEHR data without inventing
		its own idea of what a health record is.
	</p>
	<div class="hero-actions">
		<a class="button-primary" href="/crates/openehr/">Read the crate documentation</a>
		<a class="button-secondary" href="/spec/">Read the specification</a>
	</div>
</section>

<section class="section" aria-labelledby="core-heading">
	<h2 id="core-heading">The core crate</h2>
	{#if core}
		<div class="card-grid card-grid-single">
			<Card class="crate-card" heading={core.title} headingLevel={3} href={core.route}>
				<p>{core.summary}</p>
				<!-- Hand-maintained, not derived from data.crates -- docs.js parses no
				     version field. This is the same shape of bug as W-19
				     (spec/audit.md): a dependency snippet nothing checks. Keep it in
				     sync with agents/publishing.md's local version by hand until this
				     is data-driven. -->
				<p class="crate-install"><code>openehr = "0.8"</code></p>
			</Card>
		</div>
	{/if}
</section>

<section class="section" aria-labelledby="persistence-heading">
	<h2 id="persistence-heading">Persistence</h2>
	<p class="section-lede">
		One engine-agnostic crate holds the storage model, the projection onto rows, the commit rules,
		and the conformance suite. Each database crate states how much of that suite it has actually
		run — <strong>Store</strong> means a working store verified against a real database,
		<strong>Schema</strong> means the DDL has been executed by a real server, and
		<strong>Dialect</strong> means the crate emits DDL that no server of that kind has yet seen.
	</p>
	<div class="card-grid">
		{#each persistence as crate (crate.name)}
			<Card class="crate-card" heading={crate.title} headingLevel={3} href={crate.route}>
				{#if crate.conformance}
					<Badge
						class="crate-badge"
						type={badgeType(crate.conformance)}
						label={`Conformance level: ${crate.conformance}`}
					>
						{crate.conformance}
					</Badge>
				{/if}
				<p>{crate.summary}</p>
			</Card>
		{/each}
	</div>
</section>

<section class="section" aria-labelledby="spec-heading">
	<h2 id="spec-heading">Specification-driven</h2>
	<p class="section-lede">
		The specification is normative, and it is published here in full — {data.specCount} documents.
		Every requirement has a permanent identifier cited from the code, the tests, and the
		documentation, so a claim about these crates is traceable back to a decision. The
		<a href="/spec/conformance-matrix/">conformance matrix</a> records what is verified today, and
		the <a href="/spec/audit/">audit</a> records every known gap with its evidence.
	</p>
	<div class="section-links">
		<a href="/spec/01-scope/">What is excluded, and why</a>
		<a href="/spec/conformance-matrix/">Conformance matrix</a>
		<a href="/spec/audit/">Audit</a>
	</div>
</section>

<section class="section" aria-labelledby="source-heading">
	<h2 id="source-heading">Source</h2>
	<p class="section-lede">
		This site renders the crates' own documentation. The crates are the source of truth.
	</p>
	<div class="section-links">
		<a href={REPOSITORIES.core}>github.com/openehr-rust/openehr-rust</a>
	</div>
</section>
