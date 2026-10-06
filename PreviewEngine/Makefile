# Rust embedded preview helpers. Outputs stay outside the source tree.
# A pinned nightly supports the translated engine's C variadic ABI.
# C++ remains only at the upstream font-layout/platform font boundary.
#
#   make -C PreviewEngine BUILD_DIR=/abs/out [TECKIT_CFLAGS=… TECKIT_LIBS=…]
#
# Runtime libraries remain dynamically linked, including TECkit.

BUILD_DIR ?= $(if $(TMPDIR),$(TMPDIR),/tmp)/pitex-preview-engine-build
CARGO ?= cargo
PKG_CONFIG ?= pkg-config
OPT ?= -O2 -g

ifeq ($(shell uname),Darwin)
BREW ?= $(shell command -v brew >/dev/null 2>&1 && brew --prefix)
ICU_PREFIX ?= $(if $(BREW),$(shell brew --prefix icu4c 2>/dev/null))
ifneq ($(ICU_PREFIX),)
export PKG_CONFIG_PATH := $(ICU_PREFIX)/lib/pkgconfig:$(PKG_CONFIG_PATH)
endif
endif

export PKG_CONFIG
export PITEX_PREVIEW_NATIVE_OPT := $(OPT)
# Explicit caller overrides bypass TECkit's optional pkg-config file.
ifneq ($(origin TECKIT_CFLAGS),undefined)
export TECKIT_CFLAGS
endif
ifneq ($(origin TECKIT_LIBS),undefined)
export TECKIT_LIBS
endif

CARGO_TARGET_DIR := $(abspath $(BUILD_DIR))/cargo
export CARGO_TARGET_DIR

all:
	$(CARGO) build --release --locked --bin pitex-preview --bin pitex-preview-xetex
	@mkdir -p "$(BUILD_DIR)/bin"
	cp "$(CARGO_TARGET_DIR)/release/pitex-preview" "$(CARGO_TARGET_DIR)/release/pitex-preview-xetex" "$(BUILD_DIR)/bin/"

selftest:
	$(CARGO) build --release --locked --bin selftest
	cp "$(CARGO_TARGET_DIR)/release/selftest" "$(BUILD_DIR)/selftest"
	"$(BUILD_DIR)/selftest"

clean:
	rm -rf "$(BUILD_DIR)"

.PHONY: all selftest clean
