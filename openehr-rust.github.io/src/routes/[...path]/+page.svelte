<script>
	import {
		ArticleLayout,
		BreadcrumbNav,
		BreadcrumbList,
		BreadcrumbListItem,
		ContentsNav,
		ContentsList,
		ContentsListItem,
		PaginationNav,
		PaginationList,
		PaginationListItem
	} from 'lily-design-system-svelte-headless';
	import { REPOSITORIES, SITE_URL } from '$lib/site.js';

	let { data } = $props();

	const doc = $derived(data.doc);
	const url = $derived(`${SITE_URL}${doc.route}`);

	// Where this page's Markdown actually lives upstream. The site flattens the
	// two repositories into one URL space, so the link back has to be rebuilt
	// rather than derived from the route.
	const source = $derived.by(() => {
		const file = doc.file;
		if (file.startsWith('spec/')) return `${REPOSITORIES.core}/blob/main/${file}`;
		if (file === 'crates/openehr.md') return `${REPOSITORIES.core}/blob/main/README.md`;
		if (file === 'crates/openehr-store-conformance.md') {
			return `${REPOSITORIES.databases}/blob/main/openehr-store/spec/conformance.md`;
		}
		const crate = /^crates\/(.+)\.md$/.exec(file)[1];
		return `${REPOSITORIES.databases}/blob/main/${crate}/README.md`;
	});
</script>

<svelte:head>
	<title>{data.title}</title>
	<meta name="description" content={doc.summary} />
	<link rel="canonical" href={url} />
	<meta property="og:title" content={doc.title} />
	<meta property="og:description" content={doc.summary} />
	<meta property="og:type" content="article" />
	<meta property="og:url" content={url} />
</svelte:head>

<BreadcrumbNav label="Breadcrumb" class="doc-breadcrumb">
	<BreadcrumbList>
		<BreadcrumbListItem><a href="/">Overview</a></BreadcrumbListItem>
		<BreadcrumbListItem><a href={doc.section.route}>{doc.section.label}</a></BreadcrumbListItem>
		<BreadcrumbListItem current>{doc.title}</BreadcrumbListItem>
	</BreadcrumbList>
</BreadcrumbNav>

{#if doc.headings.length > 2}
	<ContentsNav label="On this page" class="doc-contents">
		<h2>On this page</h2>
		<ContentsList>
			{#each doc.headings as heading (heading.id)}
				<ContentsListItem data-depth={heading.depth}>
					<a href={`#${heading.id}`}>{heading.text}</a>
				</ContentsListItem>
			{/each}
		</ContentsList>
	</ContentsNav>
{/if}

<ArticleLayout label={doc.title} class="prose">
	<!-- `{@html}` is safe here and only here: doc.html is produced at build time
	     by src/lib/markdown.js from the Markdown vendored into content/, which
	     comes from this organization's own repositories. Nothing user-supplied
	     and nothing fetched at runtime reaches this expression. -->
	{@html doc.html}
</ArticleLayout>

{#if doc.previous || doc.next}
	<PaginationNav label="Document navigation" class="doc-pagination">
		<PaginationList>
			{#if doc.previous}
				<PaginationListItem>
					<a href={doc.previous.route} rel="prev">
						<span class="direction">Previous</span>
						<span class="title">{doc.previous.title}</span>
					</a>
				</PaginationListItem>
			{/if}
			{#if doc.next}
				<PaginationListItem>
					<a href={doc.next.route} rel="next">
						<span class="direction">Next</span>
						<span class="title">{doc.next.title}</span>
					</a>
				</PaginationListItem>
			{/if}
		</PaginationList>
	</PaginationNav>
{/if}

<p class="doc-source">
	<a href={source}>View this page's source on GitHub</a> — the crates are the source of truth; this
	site renders them.
</p>
