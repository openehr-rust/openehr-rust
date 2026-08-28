<script>
	import ArticleLayout from '$lib/lily/components/ArticleLayout.svelte';
	import BreadcrumbNav from '$lib/lily/components/BreadcrumbNav.svelte';
	import BreadcrumbList from '$lib/lily/components/BreadcrumbList.svelte';
	import BreadcrumbListItem from '$lib/lily/components/BreadcrumbListItem.svelte';
	import { SITE_NAME, SITE_URL } from '$lib/site.js';

	let { data } = $props();

	const description = 'Every crate published by openehr-rust, with what each one is verified to do.';
</script>

<svelte:head>
	<title>Crates — {SITE_NAME}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={`${SITE_URL}/crates/`} />
	<meta property="og:title" content={`Crates — ${SITE_NAME}`} />
	<meta property="og:description" content={description} />
	<meta property="og:type" content="website" />
	<meta property="og:url" content={`${SITE_URL}/crates/`} />
</svelte:head>

<BreadcrumbNav label="Breadcrumb" class="doc-breadcrumb">
	<BreadcrumbList>
		<BreadcrumbListItem><a href="/">Overview</a></BreadcrumbListItem>
		<BreadcrumbListItem current>Crates</BreadcrumbListItem>
	</BreadcrumbList>
</BreadcrumbNav>

<ArticleLayout label="Crates" class="prose">
	<h1>Crates</h1>
	<p>{description}</p>

	<table>
		<thead>
			<tr>
				<th scope="col">Crate</th>
				<th scope="col">Conformance</th>
				<th scope="col">What it is</th>
			</tr>
		</thead>
		<tbody>
			{#each data.crates as crate (crate.name)}
				<tr>
					<th scope="row"><a href={crate.route}><code>{crate.name}</code></a></th>
					<td>{crate.conformance ?? '—'}</td>
					<td>{crate.summary}</td>
				</tr>
			{/each}
		</tbody>
	</table>

	<h2>Reading the conformance column</h2>
	<p>
		The column is the crate's own claim, lifted from its README so that it is visible without
		opening all six pages.
	</p>
	<ul>
		<li><strong>Store</strong> — a working store, verified against a real database.</li>
		<li><strong>Schema</strong> — the DDL has been executed by a real server of that kind.</li>
		<li>
			<strong>Dialect</strong> — the crate emits DDL, and no server of that kind has ever run it.
			There is no store, no driver dependency, and no connection handling.
		</li>
	</ul>
	<p>
		The shared suite behind those levels is
		<a href="/crates/openehr-store/conformance/">openehr-store's conformance specification</a>.
	</p>
</ArticleLayout>
