# Publishing the GitHub Pages site is documented in
# spec/monorepo-github-pages/index.md and openehr-rust.github.io/README.md
# ("Deploy"): openehr-rust.github.io/ lives in this monorepo, but GitHub Pages
# for an organization site has to be served from a repository literally named
# openehr-rust.github.io, and GitHub Actions never discovers a
# .github/workflows/ nested under a monorepo subdirectory. So the site is
# published by rewriting that subdirectory's history onto a separate
# sibling repository via `git subtree`.
#
# One-time setup, per checkout (a remote is local git config, not something
# this repository can commit):
#   git remote add github-pages git@github.com:openehr-rust/openehr-rust.github.io.git
#
# `git subtree push` (unlike `git subtree split -b ... && git push --force`,
# what scripts/publish-pages-subtree.py does) pushes incrementally: it only
# works because that sibling's main is already a subtree-split descendant of
# this monorepo's history, which it became on 2026-08-31, when
# scripts/publish-pages-subtree.py --push first bootstrapped it. Pushed from a
# checkout that has never done that bootstrap, this command fails asking for
# a merge instead of silently rewriting history -- which is the tradeoff
# against the script: safer on every push after the first, unusable for it.
.PHONY: github-pages
github-pages:
	git subtree push --prefix=openehr-rust.github.io github-pages main
