BINARY = rotator_protocol
VERSION = $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
BUILD_DIR = build

# Feature flags
FEATURES =
ifdef GUI
	FEATURES = --features gui
	SUFFIX = -gui
endif

.PHONY: all
all: release

# === Local builds ===

.PHONY: release
release:
	cargo build --release $(FEATURES)

.PHONY: debug
debug:
	cargo build $(FEATURES)

.PHONY: check
check:
	cargo check
	cargo clippy -- -D warnings

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: test
test:
	cargo test

.PHONY: clean
clean:
	cargo clean
	rm -rf $(BUILD_DIR)

# === Cross-platform targets ===

.PHONY: setup-targets
setup-targets:
	rustup target add x86_64-unknown-linux-gnu
	rustup target add x86_64-unknown-linux-musl
	rustup target add aarch64-unknown-linux-gnu
	rustup target add armv7-unknown-linux-gnueabihf
	rustup target add x86_64-pc-windows-gnu
	rustup target add x86_64-apple-darwin
	rustup target add aarch64-apple-darwin

# Linux
.PHONY: linux-x64
linux-x64:
	cargo build --release --target x86_64-unknown-linux-gnu $(FEATURES)
	@mkdir -p $(BUILD_DIR)
	cp target/x86_64-unknown-linux-gnu/release/$(BINARY) $(BUILD_DIR)/$(BINARY)-linux-x64$(SUFFIX)

.PHONY: linux-musl
linux-musl:
	cargo build --release --target x86_64-unknown-linux-musl $(FEATURES)
	@mkdir -p $(BUILD_DIR)
	cp target/x86_64-unknown-linux-musl/release/$(BINARY) $(BUILD_DIR)/$(BINARY)-linux-musl$(SUFFIX)

.PHONY: linux-arm64
linux-arm64:
	cargo build --release --target aarch64-unknown-linux-gnu $(FEATURES)
	@mkdir -p $(BUILD_DIR)
	cp target/aarch64-unknown-linux-gnu/release/$(BINARY) $(BUILD_DIR)/$(BINARY)-linux-arm64$(SUFFIX)

.PHONY: linux-armv7
linux-armv7:
	cargo build --release --target armv7-unknown-linux-gnueabihf $(FEATURES)
	@mkdir -p $(BUILD_DIR)
	cp target/armv7-unknown-linux-gnueabihf/release/$(BINARY) $(BUILD_DIR)/$(BINARY)-linux-armv7$(SUFFIX)

# Windows
.PHONY: windows
windows:
	cargo build --release --target x86_64-pc-windows-gnu $(FEATURES)
	@mkdir -p $(BUILD_DIR)
	cp target/x86_64-pc-windows-gnu/release/$(BINARY).exe $(BUILD_DIR)/$(BINARY)-windows$(SUFFIX).exe

# macOS
.PHONY: macos-x64
macos-x64:
	cargo build --release --target x86_64-apple-darwin $(FEATURES)
	@mkdir -p $(BUILD_DIR)
	cp target/x86_64-apple-darwin/release/$(BINARY) $(BUILD_DIR)/$(BINARY)-macos-x64$(SUFFIX)

.PHONY: macos-arm64
macos-arm64:
	cargo build --release --target aarch64-apple-darwin $(FEATURES)
	@mkdir -p $(BUILD_DIR)
	cp target/aarch64-apple-darwin/release/$(BINARY) $(BUILD_DIR)/$(BINARY)-macos-arm64$(SUFFIX)

# === Batch builds ===

.PHONY: linux
linux: linux-x64 linux-musl linux-arm64 linux-armv7

.PHONY: macos
macos: macos-x64 macos-arm64

.PHONY: desktop
desktop: linux-x64 windows macos-x64 macos-arm64

.PHONY: all-platforms
all-platforms: linux windows macos

# === Help ===

.PHONY: help
help:
	@echo "Rotator Protocol Build System"
	@echo ""
	@echo "Usage: make [target] [GUI=1]"
	@echo ""
	@echo "Add GUI=1 to enable GUI feature, e.g.:"
	@echo "  make release GUI=1"
	@echo "  make windows GUI=1"
	@echo ""
	@echo "Local builds:"
	@echo "  release          - Release build"
	@echo "  debug            - Debug build"
	@echo "  check            - Check code (cargo check + clippy)"
	@echo "  test             - Run tests"
	@echo "  clean            - Clean build"
	@echo ""
	@echo "Cross-platform (single):"
	@echo "  linux-x64        - Linux x86_64 (glibc)"
	@echo "  linux-musl       - Linux x86_64 (musl, static)"
	@echo "  linux-arm64      - Linux ARM64"
	@echo "  linux-armv7      - Linux ARMv7 (Raspberry Pi)"
	@echo "  windows          - Windows x86_64"
	@echo "  macos-x64        - macOS x86_64"
	@echo "  macos-arm64      - macOS ARM64 (Apple Silicon)"
	@echo ""
	@echo "Cross-platform (batch):"
	@echo "  linux            - All Linux targets"
	@echo "  macos            - All macOS targets"
	@echo "  desktop          - Linux x64 + Windows + macOS"
	@echo "  all-platforms    - All platforms"
	@echo ""
	@echo "Setup:"
	@echo "  setup-targets    - Install rustup cross-compile targets"
	@echo ""
	@echo "Output: $(BUILD_DIR)/"
