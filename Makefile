PREFIX ?= $(HOME)/.local
APP_ID := com.niconex.Vibes

# Cross-build settings. Debian trixie is the oldest base with libadwaita 1.7, which the settings
# window's AdwToggleGroup needs.
CONTAINER ?= $(shell command -v docker || command -v podman)
CROSS_PLATFORM ?= linux/arm64
CROSS_IMAGE ?= rust:1-trixie
CROSS_OUT ?= vibes-linux-arm64

.PHONY: all run test check preview icons install uninstall clean linux-arm64

# The binary, dropped in the repository root — same place the Android Makefile left its apk.
all: vibes

vibes: $(shell find src -type f) Cargo.toml
	cargo build --release
	cp target/release/vibes vibes

run:
	cargo run --release

test:
	cargo test

check:
	cargo fmt --check
	cargo test

# Offscreen PNGs of the painted layer, into /tmp — look at the waves without launching anything.
preview:
	cargo run --example render

icons:
	python3 data/icons/make-icons.py

# A binary for 64-bit ARM Linux — Raspberry Pi, PinePhone, Librem 5, ARM servers.
#
# GTK cannot be cross-linked from a foreign host without a full target sysroot: gtk4-sys asks
# pkg-config for the target's GTK, libadwaita and ALSA, and none of that comes from rustup. So
# the build runs inside an arm64 Linux container instead, which needs no sysroot to assemble.
# On Apple Silicon that container runs natively, not emulated.
#
# Override CROSS_PLATFORM (and CROSS_OUT) for another architecture:
#   make linux-arm64 CROSS_PLATFORM=linux/amd64 CROSS_OUT=vibes-linux-amd64
#
# The cargo registry and the target directory live in named volumes, so only the first build
# pays for the downloads.
linux-arm64:
	@test -n "$(CONTAINER)" || { \
		echo "No docker or podman found. Either install one, or build on an ARM Linux machine with plain \`make\`."; \
		exit 1; }
	$(CONTAINER) run --rm --platform $(CROSS_PLATFORM) \
		-v "$(CURDIR)":/src -w /src \
		-v vibes-cross-registry:/usr/local/cargo/registry \
		-v vibes-cross-target:/target \
		$(CROSS_IMAGE) sh -c '\
			apt-get update -qq && \
			apt-get install -y -qq --no-install-recommends \
				pkg-config libgtk-4-dev libadwaita-1-dev libasound2-dev && \
			CARGO_TARGET_DIR=/target cargo build --release && \
			cp /target/release/vibes /src/$(CROSS_OUT)'
	@echo "-> $(CROSS_OUT)"

# ponytail: mkdir + cp rather than `install -D`, which BSD install does not have.
install: vibes
	mkdir -p $(DESTDIR)$(PREFIX)/bin
	cp vibes $(DESTDIR)$(PREFIX)/bin/vibes
	chmod 755 $(DESTDIR)$(PREFIX)/bin/vibes
	mkdir -p $(DESTDIR)$(PREFIX)/share/applications
	cp data/$(APP_ID).desktop $(DESTDIR)$(PREFIX)/share/applications/
	mkdir -p $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps
	cp data/icons/$(APP_ID).svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/
	mkdir -p $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps
	cp data/icons/$(APP_ID)-symbolic.svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps/
	-gtk-update-icon-cache -qtf $(DESTDIR)$(PREFIX)/share/icons/hicolor
	-update-desktop-database -q $(DESTDIR)$(PREFIX)/share/applications

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/vibes
	rm -f $(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg

clean:
	cargo clean
	rm -f vibes vibes-linux-*
