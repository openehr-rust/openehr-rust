<script>
	import { page } from '$app/state';
	import { SkipLink, Header, Footer } from 'lily-design-system-svelte-headless';
	import { SharePicker } from 'lily-design-system-svelte-share-picker';
	import { ThemePicker } from 'lily-design-system-svelte-theme-picker';
	import { TextSizePicker } from 'lily-design-system-svelte-text-size-picker';
	import { ORGANIZATION, REPOSITORIES, SITE_NAME, THEMES, THEME_LABELS } from '$lib/site.js';
	import '../styles/site.css';

	let { children } = $props();

	const links = [
		{ href: '/', label: 'Overview' },
		{ href: '/crates/', label: 'Crates' },
		{ href: '/spec/', label: 'Specification' }
	];

	// SharePicker's own docs are explicit that which networks to offer is an
	// editorial call the package won't make for you -- it ships no endpoints.
	// This is that call: four communities a Rust/openEHR crate's readers
	// plausibly use, not an attempt at a complete list. `href` receives
	// (url, title, text); LinkedIn's URL is the package's own quick-start
	// example verbatim. Mastodon has no single share endpoint -- it's
	// federated -- so, also per the package's example, this targets
	// mastodon.social specifically; a reader on another instance gets a
	// working share dialog there and then has to re-paste it into their own.
	const shareTargets = [
		{
			id: 'linkedin',
			label: 'LinkedIn',
			href: (url) => `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`
		},
		{
			id: 'mastodon',
			label: 'Mastodon',
			href: (url, title) =>
				`https://mastodon.social/share?text=${encodeURIComponent(title)}%20${encodeURIComponent(url)}`
		},
		{
			id: 'bluesky',
			label: 'Bluesky',
			href: (url, title) => `https://bsky.app/intent/compose?text=${encodeURIComponent(`${title} ${url}`)}`
		},
		{
			id: 'reddit',
			label: 'Reddit',
			href: (url, title) =>
				`https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`
		}
	];

	// A section link stays current for every page beneath it.
	const current = (href) => {
		const path = page.url.pathname;
		if (href === '/') return path === '/' ? 'page' : undefined;
		return path === href || path.startsWith(href) ? 'page' : undefined;
	};

	// The page.data.title convention: every route's load returns the exact
	// string it puts in <svelte:head><title> as `title` (see +page.server.js
	// at the site root). +error.svelte has no load and so no page.data.title
	// -- SITE_NAME is an honest fallback there, not a stand-in for the error
	// heading, since nobody sharing a broken link needs it worded precisely.
	const shareTitle = $derived(page.data.title ?? SITE_NAME);
</script>

<svelte:head>
	<!-- Every theme preloaded as its own stylesheet, per ThemePicker's own
	     "Preloading for zero-flicker switching" doc. Each file scopes its
	     rules to :root[data-theme="<slug>"], so with all six already present
	     a switch is the attribute change on <html> below -- no fetch, no
	     flash of unstyled content while the new theme's CSS loads. The
	     tradeoff is real and stated plainly rather than left for someone to
	     notice in a network tab: six ~24 kB-gzipped stylesheets (~145 kB
	     total) load upfront instead of one, because each inlines the CSS for
	     every Lily component the theme covers. src/app.html's own managed
	     link (data-lily-theme-picker="theme") is what ThemePicker actually
	     swaps; that fetch becomes a same-URL cache hit once the matching
	     preload below has already landed. -->
	{#each THEMES as theme (theme)}
		<link rel="stylesheet" href={`/themes/${theme}.css`} />
	{/each}
</svelte:head>

<SkipLink href="#main" label="Skip to main content" />

<Header label="Site header" class="site-header">
	<div class="site-header-inner">
		<a class="site-brand" href="/">
			<img src="/icon.svg" alt="" aria-hidden="true" width="32" height="32" />
			<span>{SITE_NAME}</span>
		</a>
		<nav class="site-nav" aria-label="Main">
			{#each links as link (link.href)}
				<a href={link.href} aria-current={current(link.href)}>{link.label}</a>
			{/each}
			<a href={ORGANIZATION}>GitHub</a>
		</nav>
		<div class="site-tools">
			<!-- `targets` are shareTargets above. `url` defaults to the current
			     page, read at share time, so this needs no per-route wiring.
			     `title` comes from the page.data.title convention (see shareTitle
			     above). Where the platform has a native share sheet, `strategy`
			     defaulting to "auto" opens that instead of this list. -->
			<SharePicker
				label="Share this page"
				title={shareTitle}
				targets={shareTargets}
				copyLabel="Copy link"
				copiedLabel="Link copied"
				copyFailedLabel="Could not copy — copy it from the address bar"
			/>
			<TextSizePicker
				label="Text size"
				sizes={['small', 'medium', 'large', 'x-large']}
				storageKey="openehr-rust-text-size"
			/>
			<ThemePicker
				label="Theme"
				themesUrl="/themes/"
				themes={THEMES}
				themeLabels={THEME_LABELS}
				storageKey="openehr-rust-theme"
				detectFromSystem
			/>
		</div>
	</div>
</Header>

<main id="main" class="site-main">
	{@render children()}
</main>

<Footer label="Site footer" class="site-footer">
	<div class="site-footer-inner">
		<p>
			<em>{SITE_NAME}</em> — openEHR Reference Model types, validation, paths, AQL parsing, change
			control, and persistence, as Rust crates. Licensed MIT OR Apache-2.0. Built with the
			<a href="https://github.com/LilyDesignSystem">Lily Design System</a>. openEHR specifications
			are published by the
			<a href="https://openehr.org/">openEHR Foundation</a>; these crates are an independent
			implementation and are not endorsed by or affiliated with the Foundation.
		</p>
		<div class="site-footer-links">
			<a href={ORGANIZATION}>GitHub</a>
			<a href={REPOSITORIES.core}>openehr-rust</a>
			<a href="/spec/">Specification</a>
		</div>
	</div>
</Footer>
