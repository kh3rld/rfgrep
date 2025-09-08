# Makefile for rfgrep man pages
# Usage: make install (requires sudo for system-wide installation)

PREFIX ?= /usr/local
MANDIR ?= $(PREFIX)/share/man
MAN1DIR = $(MANDIR)/man1

MANPAGES = rfgrep.1 rfgrep-search.1 rfgrep-interactive.1 rfgrep-list.1 rfgrep-completions.1 rfgrep-plugins.1 rfgrep-tui.1

.PHONY: all install uninstall clean check gzip

all: $(MANPAGES)

gzip: $(MANPAGES)
	@echo "Gzipping man pages..."
	@for page in $(MANPAGES); do \
		echo "Gzipping $$page..."; \
		gzip -c $$page > $$page.gz; \
	done

install: $(MANPAGES)
	@echo "Installing man pages to $(MAN1DIR)..."
	@mkdir -p $(MAN1DIR)
	@for page in $(MANPAGES); do \
		echo "Installing $$page..."; \
		cp $$page $(MAN1DIR)/; \
		gzip -f $(MAN1DIR)/$$page; \
	done
	@echo "Man pages installed successfully!"
	@echo "You can now use: man rfgrep"

uninstall: 
	@echo "Removing man pages from $(MAN1DIR)..."
	@for page in $(MANPAGES); do \
		echo "Removing $$page..."; \
		rm -f $(MAN1DIR)/$$page.gz; \
	done
	@echo "Man pages removed successfully!"

clean:
	@echo "Cleaning up..."
	@rm -f *.gz

check:
	@echo "Checking man page syntax..."
	@for page in $(MANPAGES); do \
		echo "Checking $$page..."; \
		man --warnings --local-file $$page > /dev/null 2>&1 || echo "Warning: $$page may have syntax issues"; \
	done

# User installation (no sudo required)
install-user: $(MANPAGES)
	@echo "Installing man pages to user directory..."
	@mkdir -p $(HOME)/.local/share/man/man1
	@for page in $(MANPAGES); do \
		echo "Installing $$page..."; \
		cp $$page $(HOME)/.local/share/man/1/; \
		gzip -f $(HOME)/.local/share/man/man1/$$page; \
	done
	@echo "Man pages installed to user directory!"
	@echo "Add to your shell profile: export MANPATH=\\$$MANPATH:$(HOME)/.local/share/man"

# Development helpers
preview: $(MANPAGES)
	@echo "Previewing man pages..."
	@for page in $(MANPAGES); do \
		echo "=== $$page ==="; \
		man --local-file $$page | head -20; \
		echo; \
	done

validate:
	@echo "Validating man page format..."
	@for page in $(MANPAGES); do \
		echo "Validating $$page..."; \
		mandoc -T lint $$page || echo "Warning: $$page has format issues"; \
	done

help:
	@echo "Available targets:"
	@echo "  install      - Install man pages system-wide (requires sudo)"
	@echo "  install-user - Install man pages to user directory"
	@echo "  uninstall    - Remove man pages from system"
	@echo "  clean        - Remove generated files"
	@echo "  check        - Check man page syntax"
	@echo "  preview      - Preview man pages"
	@echo "  validate     - Validate man page format"
	@echo "  help         - Show this help message"