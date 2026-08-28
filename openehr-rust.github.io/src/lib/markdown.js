import { Marked } from 'marked';
import GithubSlugger from 'github-slugger';
import { rewriteHref } from './paths.js';

const escapeAttribute = (value) =>
	value.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');

const plainText = (html) =>
	html
		.replace(/<[^>]*>/g, '')
		.replace(/&amp;/g, '&')
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>')
		.replace(/&quot;/g, '"')
		.replace(/&#39;/g, "'")
		.trim();

/**
 * Render one content file.
 *
 * @param {string} markdown raw file contents
 * @param {object} options
 * @param {string} options.file content path, used to resolve relative links
 * @returns {{ html: string, title: string, headings: Array<{depth: number, id: string, text: string}>, summary: string }}
 */
export function renderMarkdown(markdown, { file }) {
	const slugger = new GithubSlugger();
	const headings = [];
	let title = '';

	const marked = new Marked({ gfm: true });
	marked.use({
		renderer: {
			heading({ tokens, depth }) {
				const html = this.parser.parseInline(tokens);
				const text = plainText(html);
				const id = slugger.slug(text);
				if (depth === 1 && !title) title = text;
				if (depth === 2 || depth === 3) headings.push({ depth, id, text });
				// The page title needs no self-anchor.
				const anchor =
					depth === 1
						? ''
						: `<a class="heading-anchor" href="#${id}" aria-label="Link to “${escapeAttribute(text)}”">#</a>`;
				return `<h${depth} id="${id}">${html}${anchor}</h${depth}>\n`;
			},
			link({ href, title: linkTitle, tokens }) {
				const html = this.parser.parseInline(tokens);
				const resolved = rewriteHref(href, file);
				const external = /^[a-z][a-z0-9+.-]*:/i.test(resolved) || resolved.startsWith('//');
				const attributes = [
					`href="${escapeAttribute(resolved)}"`,
					linkTitle ? `title="${escapeAttribute(linkTitle)}"` : '',
					external ? 'rel="noopener noreferrer"' : ''
				].filter(Boolean);
				return `<a ${attributes.join(' ')}>${html}</a>`;
			},
			code({ text, lang }) {
				// No highlighter: a crate's README is mostly Rust, and shipping a
				// tokenizer to colour it costs more than it returns. The language
				// stays on the element so a future stylesheet can use it.
				const language = (lang ?? '').split(/\s+/)[0];
				const className = language ? ` class="language-${escapeAttribute(language)}"` : '';
				const escaped = text
					.replace(/&/g, '&amp;')
					.replace(/</g, '&lt;')
					.replace(/>/g, '&gt;');
				return `<pre><code${className}>${escaped}</code></pre>\n`;
			}
		}
	});

	const html = marked.parse(markdown);
	return { html, title, headings, summary: summarize(markdown) };
}

/**
 * The document's opening paragraph, as one line.
 *
 * Used for <meta name="description"> and for the crate summaries on the
 * overview. It has to join the paragraph rather than take the first line:
 * these files are hard-wrapped at about 78 columns, so a first line alone
 * ends mid-sentence.
 */
export function summarize(markdown) {
	const skip = (text) =>
		!text ||
		text.startsWith('#') ||
		text.startsWith('---') ||
		text.startsWith('>') ||
		text.startsWith('```') ||
		text.startsWith('|') ||
		text.startsWith('- ') ||
		text.startsWith('* ');

	const lines = markdown.split('\n');
	for (let i = 0; i < lines.length; i++) {
		if (skip(lines[i].trim())) continue;
		// Take the whole paragraph: everything up to the next blank line.
		const paragraph = [];
		for (let j = i; j < lines.length; j++) {
			const text = lines[j].trim();
			if (!text || skip(text)) break;
			paragraph.push(text);
		}
		const plain = paragraph
			.join(' ')
			.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
			.replace(/[*_`]/g, '')
			.replace(/\s+/g, ' ')
			.trim();
		if (plain.length < 20) continue;
		return plain.length > 300 ? `${plain.slice(0, 297).trimEnd()}…` : plain;
	}
	return '';
}
