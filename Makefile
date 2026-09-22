PREFIX ?= $(HOME)/.local
APP_ID := com.niconex.Vibes

# Cross-build settings. Debian trixie is the oldest base with libadwaita 1.7, which the settings
# window's AdwToggleGroup needs.
CONTAINER ?= $(shell command -v podman || command -v docker)
CROSS_PLATFORM ?= linux/arm64
CROSS_IMAGE ?= rust:1-trixie
CROSS_OUT ?= vibes-linux-arm64
PMOS_PLATFORM ?= linux/arm64
# musl gives up on a name when a router answers the AAAA query with NXDOMAIN, which glibc
# shrugs off, so the Alpine build asks a public resolver. Point it elsewhere if you prefer.
PMOS_DNS ?= 1.1.1.1

VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DMG ?= vibes-$(VERSION)-macos.dmg

# Honours CARGO_TARGET_DIR, so a container build installs its own binary, not the host's.
BIN := $(or $(CARGO_TARGET_DIR),target)/release/vibes

.PHONY: all run test check preview icons install uninstall clean linux-arm64 arch postmarketos macos dmg container-check

# The binary, dropped in the repository root — same place the Android Makefile left its apk.
all: vibes

vibes: $(BIN)
	cp -f $(BIN) vibes

$(BIN): $(shell find src -type f) Cargo.toml
	cargo build --release

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

# Both container targets need more than a binary on PATH: on macOS podman runs the containers
# inside a VM, and `podman machine start` is a separate step that survives no reboot, so the
# plain `command -v` check passed while every build died with "unable to connect to Podman socket".
container-check:
	@test -n "$(CONTAINER)" || { \
		echo "No docker or podman found. Either install one, or build on an ARM Linux machine with plain \`make\`."; \
		exit 1; }
	@$(CONTAINER) info >/dev/null 2>&1 || { \
		echo "$(CONTAINER) is installed but not reachable. If it is podman on macOS: podman machine start"; \
		exit 1; }

# A binary for 64-bit ARM Linux — Raspberry Pi, PinePhone, Librem 5, ARM servers.
#
# GTK cannot be cross-linked from a foreign host without a full target sysroot: gtk4-sys asks
# pkg-config for the target's GTK, libadwaita and ALSA, and none of that comes from rustup. So
# the build runs inside an arm64 Linux container instead, which needs no sysroot to assemble.
# On Apple Silicon that container runs natively, not emulated. On an x86_64 Linux host it runs
# under QEMU, which needs qemu-user-static registered with binfmt_misc (Arch:
# qemu-user-static qemu-user-static-binfmt); without it the container dies with "exec format error".
#
# Override CROSS_PLATFORM (and CROSS_OUT) for another architecture:
#   make linux-arm64 CROSS_PLATFORM=linux/amd64 CROSS_OUT=vibes-linux-amd64
#
# The cargo registry and the target directory live in named volumes, so only the first build
# pays for the downloads.
linux-arm64: container-check
	$(CONTAINER) run --rm --platform $(CROSS_PLATFORM) \
		-v "$(CURDIR)":/src -w /src \
		-v vibes-cross-registry:/usr/local/cargo/registry \
		-v vibes-cross-target:/target \
		$(CROSS_IMAGE) sh -c '\
			apt-get update -qq && \
			apt-get install -y -qq --no-install-recommends \
				pkg-config libgtk-4-dev libadwaita-1-dev libasound2-dev \
				libpipewire-0.3-dev libclang-dev && \
			CARGO_TARGET_DIR=/target cargo build --release && \
			cp /target/release/vibes /src/$(CROSS_OUT)'
	@echo "-> $(CROSS_OUT)"

# An Arch Linux package, dropped in the repository root: `sudo pacman -U vibes-*.pkg.tar.zst`.
arch:
	cd dist/arch && PKGDEST="$(CURDIR)" makepkg -f

# A postmarketOS (Alpine) package for a phone: ./vibes-<ver>-r0-<arch>.apk. Built inside an
# alpine:edge container of the phone's architecture, so it links the same musl and libraries
# pmOS ships. From an x86_64 host that needs the QEMU binfmt setup described for linux-arm64.
# Signed with a throwaway key, so install it with: apk add --allow-untrusted vibes-*.apk
#
# Compiling gtk4 needs real memory: on a default `podman machine` (2 GiB, no swap) rustc is
# SIGKILLed by the OOM killer partway through, which reads as a plain "could not compile gtk4".
# Give the VM 8 GiB first: podman machine stop && podman machine set --memory 8192 && podman machine start
postmarketos: container-check
	$(CONTAINER) run --rm --platform $(PMOS_PLATFORM) --dns $(PMOS_DNS) \
		-v "$(CURDIR)":/src -w /src/dist/postmarketos \
		-v vibes-pmos-registry:/root/.cargo/registry \
		-v vibes-pmos-target:/target \
		alpine:edge sh -c '\
			apk add -q alpine-sdk && \
			abuild-keygen -a -n -q && cp /root/.config/abuild/*.rsa.pub /etc/apk/keys/ && \
			CARGO_TARGET_DIR=/target REPODEST=/tmp/repo abuild -F -r -q && \
			for f in /tmp/repo/*/*/vibes-*.apk; do cp "$$f" "/src/$$(basename "$$f" .apk)-$$(apk --print-arch).apk"; done'
	@ls vibes-*.apk

# An application bundle, Vibes.app, in the repository root. macOS only; build host needs
# `brew install gtk4 libadwaita` (rsvg-convert, used for the icon, comes along with those).
#
# ponytail: the bundle links the Homebrew GTK it was built against instead of carrying copies of
# it, so it runs on a machine that has those formulae and nowhere else. Run dylibbundler over
# Contents/MacOS/vibes if it ever has to stand alone.
macos: $(BIN) dist/macos/Info.plist data/icons/$(APP_ID).svg
	rm -rf Vibes.app vibes.iconset
	mkdir -p Vibes.app/Contents/MacOS Vibes.app/Contents/Resources vibes.iconset
	cp $(BIN) Vibes.app/Contents/MacOS/vibes
	sed 's/@VERSION@/$(VERSION)/g' dist/macos/Info.plist > Vibes.app/Contents/Info.plist
	for s in 16 32 128 256 512; do \
		rsvg-convert -w $$s -h $$s data/icons/$(APP_ID).svg -o vibes.iconset/icon_$${s}x$${s}.png; \
		rsvg-convert -w $$((s * 2)) -h $$((s * 2)) data/icons/$(APP_ID).svg \
			-o vibes.iconset/icon_$${s}x$${s}@2x.png; \
	done
	iconutil -c icns vibes.iconset -o Vibes.app/Contents/Resources/vibes.icns
	rm -rf vibes.iconset
# Ad-hoc signature: unsigned arm64 bundles will not launch, and the microphone permission is
# remembered per signing identity, so an unsigned one re-asks after every rebuild.
	codesign --force --sign - Vibes.app
	@echo "-> Vibes.app"

# A disk image of that bundle, with the usual drag-onto-Applications shortcut beside it.
dmg: macos
	rm -rf target/dmg
	mkdir -p target/dmg
	cp -R Vibes.app target/dmg/
	ln -s /Applications target/dmg/Applications
	hdiutil create -volname Vibes -srcfolder target/dmg -ov -format UDZO -quiet $(DMG)
	rm -rf target/dmg
	@echo "-> $(DMG)"

# ponytail: mkdir + cp rather than `install -D`, which BSD install does not have.
install: $(BIN)
	mkdir -p $(DESTDIR)$(PREFIX)/bin
	cp $(BIN) $(DESTDIR)$(PREFIX)/bin/vibes
	chmod 755 $(DESTDIR)$(PREFIX)/bin/vibes
	mkdir -p $(DESTDIR)$(PREFIX)/share/applications
	cp data/$(APP_ID).desktop $(DESTDIR)$(PREFIX)/share/applications/
	mkdir -p $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps
	cp data/icons/$(APP_ID).svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/
	mkdir -p $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps
	cp data/icons/$(APP_ID)-symbolic.svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps/
# Staged installs (a package) leave the caches to the package manager's own hooks: a cache
# written into DESTDIR would collide with the one hicolor-icon-theme owns.
ifeq ($(DESTDIR),)
	-gtk-update-icon-cache -qtf $(PREFIX)/share/icons/hicolor
	-update-desktop-database -q $(PREFIX)/share/applications
endif

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/vibes
	rm -f $(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg

clean:
	cargo clean
	rm -f vibes vibes-linux-* vibes-*.pkg.tar.* vibes-*.apk vibes-*.dmg
	rm -rf Vibes.app vibes.iconset
	rm -rf dist/arch/src dist/arch/pkg
