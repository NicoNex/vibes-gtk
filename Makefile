PREFIX ?= $(HOME)/.local
APP_ID := com.niconex.Vibes

.PHONY: all run test check preview icons install uninstall clean

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
	rm -f vibes
