# Multi-stage build for `openehr-axa`, the one crate here that is a service
# rather than a library — see `AGENTS.md` for why it is versioned outside the
# eight published crates and never itself published to crates.io.
#
# # "Static", precisely
#
# `rusqlite`'s `bundled` feature (`openehr-sqlite/Cargo.toml`) compiles
# `SQLite` into the binary at build time — no `libsqlite3` is loaded at
# runtime, and none needs to be present in the final image. That closes the
# one dependency this service would otherwise need a system package for.
# What this build does **not** do is a fully static, `musl`-linked binary:
# the binary still dynamically links `glibc`, so the runtime stage is a slim
# Debian image with `glibc` present, not `scratch`. TLS is `rustls`
# throughout this crate's dependency tree (confirmed by `cargo deny check`,
# `deny.toml`'s own comments) — no `openssl` shared library to carry either.
#
# # Why two stages
#
# The builder stage needs a full Rust toolchain and a C compiler (for
# `rusqlite`'s bundled build); the runtime stage needs neither. Shipping the
# builder's own image would ship a compiler and every crate source tree next
# to a clinical record service's actual attack surface for no reason.

FROM docker.io/library/rust:1.98-bookworm AS builder

WORKDIR /build

# The crate and its two path dependencies, in the shape `cargo build` needs
# to resolve them — `openehr-axa` depends on `openehr`, `openehr-store`, and
# `openehr-sqlite` by path (see `openehr-axa/Cargo.toml`), and each of those
# is its own Cargo workspace (`CLAUDE.md`: "There is no root workspace").
# Copying only these four keeps the build's own cache invalidated only by
# what the binary can actually depend on, not by an unrelated dialect crate's
# edit.
COPY openehr ./openehr
COPY openehr-store ./openehr-store
COPY openehr-sqlite ./openehr-sqlite
COPY openehr-axa ./openehr-axa

WORKDIR /build/openehr-axa
RUN cargo build --release --bin openehr-axa

FROM docker.io/library/debian:bookworm-slim AS runtime

# `ca-certificates`: `rustls` verifies against the system trust store, and
# without it present every outbound TLS connection this service or its
# dependencies make fails closed. Nothing else here needs a package — no
# `libsqlite3`, no `libssl`, per the stage comment above. Copied from the
# builder rather than `apt-get install`ed here: `rust:*-bookworm` already
# carries the package (it is `buildpack-deps`-based), so this avoids a
# second package-index fetch, from a different mirror set, for a bundle the
# first stage already has — found to matter in practice, not just in
# principle, when the runtime stage's own `apt-get update` hit a Debian
# mirror this machine's network could not complete a transfer with, on a
# host where the identical fetch had just succeeded in the builder stage
# seconds earlier.
COPY --from=builder /etc/ssl/certs /etc/ssl/certs

# Not `root`. A clinical record service that a request can reach should not
# also be able to write anywhere its own container image can.
RUN useradd --system --create-home --home-dir /home/openehr openehr
USER openehr
WORKDIR /home/openehr

COPY --from=builder /build/openehr-axa/target/release/openehr-axa /usr/local/bin/openehr-axa

# Axa reads no configuration file: its settings are environment variables with
# defaults in `openehr_axa::app::defaults`. A release build already answers
# `production` when `APP_ENV` is unset; it is stated here so that the image's
# behaviour does not depend on how it was built. `BINDING` is the one setting
# that must differ from the default inside a container: `localhost` is the
# container's own loopback, which a published port never reaches.
ENV APP_ENV=production
ENV BINDING=0.0.0.0
# The ten-second request limit is in `app::defaults()`, not here, so it holds
# outside a container too.

# `OPENEHR_SQLITE_PATH` names a file under this volume, not the volume path
# itself — see `docker-compose.yml`, which is the piece that actually
# declares the volume and sets this variable to a path inside it.
VOLUME ["/home/openehr/data"]

EXPOSE 5150

ENTRYPOINT ["openehr-axa"]
CMD ["start"]
