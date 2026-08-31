# Publishing the GitHub Pages site is documented in
# spec/monorepo-github-pages/index.md and openehr-rust.github.io/README.md
# ("Deploy"): openehr-rust.github.io/ lives in this monorepo, but GitHub Pages
# for an organization site has to be served from a repository literally named
# openehr-rust.github.io, and GitHub Actions never discovers a
# .github/workflows/ nested under a monorepo subdirectory. So the site is
# published by rewriting that subdirectory's history onto a separate
# sibling repository via `git subtree`. The actual command, and the reasoning
# behind it (in particular why it is `git subtree push` and not the
# split-and-force-push scripts/publish-pages-subtree.py does), lives in
# bin/make-github-pages -- read that before changing this target.
.PHONY: github-pages
github-pages:
	bin/make-github-pages
