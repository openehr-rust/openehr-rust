<script>
	import { page } from '$app/state';
	import SkipLink from '$lib/lily/components/SkipLink.svelte';
	import Header from '$lib/lily/components/Header.svelte';
	import Footer from '$lib/lily/components/Footer.svelte';
	import ThemePicker from '$lib/lily/helpers/ThemePicker.svelte';
	import TextSizePicker from '$lib/lily/helpers/TextSizePicker.svelte';
	import { ORGANIZATION, REPOSITORIES, SITE_NAME, THEMES, THEME_LABELS } from '$lib/site.js';
	import '../styles/site.css';

	let { children } = $props();

	const links = [
		{ href: '/', label: 'Overview' },
		{ href: '/crates/', label: 'Crates' },
		{ href: '/spec/', label: 'Specification' }
	];

	// A section link stays current for every page beneath it.
	const current = (href) => {
		const path = page.url.pathname;
		if (href === '/') return path === '/' ? 'page' : undefined;
		return path === href || path.startsWith(href) ? 'page' : undefined;
	};
</script>

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
			<a href={REPOSITORIES.core}>openehr</a>
			<a href={REPOSITORIES.databases}>openehr-databases</a>
			<a href="/spec/">Specification</a>
		</div>
	</div>
</Footer>
