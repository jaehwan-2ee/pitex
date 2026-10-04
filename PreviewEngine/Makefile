# Pitex embedded preview engine (TeXpresso-derived). Builds two helpers:
#   pitex-preview-xetex  modified XeTeX engine (checkpointing, VFS I/O)
#   pitex-preview        session driver + XDV->PDF writer
# Output goes to $(BUILD_DIR), never into the source tree (the release
# surface gate rejects build directories in the repository).
#
#   make -C PreviewEngine BUILD_DIR=/abs/out [TECKIT_CFLAGS=… TECKIT_LIBS=…]
#
# Dependencies (all dynamically linked, see PROVENANCE.md):
#   freetype2 harfbuzz graphite2 libpng zlib icu-uc teckit [fontconfig on Linux]

BUILD_DIR ?= $(if $(TMPDIR),$(TMPDIR),/tmp)/pitex-preview-engine-build
PKG_CONFIG ?= pkg-config
OPT ?= -O2 -g
UNAME := $(shell uname)
# TECkit: the distribution's LGPL shared library, used unmodified. Without
# a system-wide -dev package, pass the headers and the runtime library
# explicitly, e.g. after `apt-get download libteckit-dev && dpkg -x`:
#   TECKIT_CFLAGS=-I<extract>/usr/include TECKIT_LIBS=/usr/lib/<triple>/libTECkit.so.0
TECKIT_CFLAGS ?= $(shell $(PKG_CONFIG) --cflags teckit)
TECKIT_LIBS ?= $(shell $(PKG_CONFIG) --libs teckit)

ifeq ($(UNAME),Darwin)
# Homebrew is optional: with a private dependency prefix, point
# PKG_CONFIG_PATH (and TECKIT_CFLAGS/TECKIT_LIBS if teckit.pc is absent)
# at it instead.
BREW ?= $(shell command -v brew >/dev/null 2>&1 && brew --prefix)
ICU_PREFIX ?= $(if $(BREW),$(shell brew --prefix icu4c 2>/dev/null))
ifneq ($(ICU_PREFIX),)
export PKG_CONFIG_PATH := $(ICU_PREFIX)/lib/pkgconfig:$(PKG_CONFIG_PATH)
endif
ENGINE_PKGS = freetype2 harfbuzz graphite2 libpng zlib icu-uc
SYS_CFLAGS = -DXETEX_MAC -fno-common $(if $(BREW),-I$(BREW)/include)
SYS_LIBS = -framework Foundation -framework CoreFoundation -framework CoreGraphics -framework CoreText -framework AppKit
SYS_ENGINE_SRC = xetex/layout/xetex-XeTeXFontMgr_Mac.mm
RPATH = -Wl,-rpath,@executable_path/../Frameworks -Wl,-rpath,@executable_path
else
ENGINE_PKGS = freetype2 harfbuzz graphite2 fontconfig libpng zlib icu-uc
SYS_CFLAGS =
SYS_LIBS =
SYS_ENGINE_SRC =
RPATH =
endif

PKG_CFLAGS := $(shell $(PKG_CONFIG) --cflags $(ENGINE_PKGS))
PKG_LIBS := $(shell $(PKG_CONFIG) --libs $(ENGINE_PKGS))

COMMON_CFLAGS = $(OPT) -fPIC -Wno-unused-result -Ishared -Ixetex/common/include $(SYS_CFLAGS)
ENGINE_CFLAGS = $(COMMON_CFLAGS) -Ixetex/include -Ixetex/layout -Ixetex/engine $(PKG_CFLAGS) $(TECKIT_CFLAGS)
ENGINE_CXXFLAGS = -std=c++17 $(ENGINE_CFLAGS)
DRIVER_CFLAGS = $(COMMON_CFLAGS) -Idriver $(shell $(PKG_CONFIG) --cflags zlib)

SHARED_SRC = shared/pitex_buf.c shared/pdfread.c shared/imginfo.c
COMMON_SRC = xetex/common/utils.c xetex/common/texlive_provider.c
ENGINE_SRC = $(wildcard xetex/engine/*.c xetex/engine/*.cpp xetex/layout/*.cpp xetex/main/*.c) $(SYS_ENGINE_SRC)
DRIVER_SRC = $(wildcard driver/*.c)

obj = $(patsubst %,$(BUILD_DIR)/obj/%.o,$(1))

ENGINE_OBJ = $(call obj,$(ENGINE_SRC) $(SHARED_SRC) $(COMMON_SRC))
DRIVER_OBJ = $(patsubst %,$(BUILD_DIR)/drv/%.o,$(DRIVER_SRC) $(SHARED_SRC) $(COMMON_SRC))

ENGINE_BIN = $(BUILD_DIR)/bin/pitex-preview-xetex
DRIVER_BIN = $(BUILD_DIR)/bin/pitex-preview

all: $(ENGINE_BIN) $(DRIVER_BIN)

$(ENGINE_BIN): $(ENGINE_OBJ)
	@mkdir -p $(dir $@)
	$(CXX) -o $@ $^ $(PKG_LIBS) $(TECKIT_LIBS) $(SYS_LIBS) -lm $(RPATH)

$(DRIVER_BIN): $(DRIVER_OBJ)
	@mkdir -p $(dir $@)
	$(CC) -o $@ $^ $(shell $(PKG_CONFIG) --libs zlib) -lm

$(BUILD_DIR)/obj/%.c.o: %.c
	@mkdir -p $(dir $@)
	$(CC) $(ENGINE_CFLAGS) -c -o $@ $<

$(BUILD_DIR)/obj/%.cpp.o: %.cpp
	@mkdir -p $(dir $@)
	$(CXX) $(ENGINE_CXXFLAGS) -c -o $@ $<

$(BUILD_DIR)/obj/%.mm.o: %.mm
	@mkdir -p $(dir $@)
	$(CXX) $(ENGINE_CXXFLAGS) -c -o $@ $<

$(BUILD_DIR)/drv/%.c.o: %.c
	@mkdir -p $(dir $@)
	$(CC) $(DRIVER_CFLAGS) -c -o $@ $<

# Parser/boundary regressions (B7–B9): driver objects minus its main.
SELFTEST_OBJ = $(filter-out $(BUILD_DIR)/drv/driver/main.c.o,$(DRIVER_OBJ))
$(BUILD_DIR)/selftest: tools/selftest.c $(SELFTEST_OBJ)
	@mkdir -p $(BUILD_DIR)/drv/tools
	$(CC) $(DRIVER_CFLAGS) -c -o $(BUILD_DIR)/drv/tools/selftest.c.o tools/selftest.c
	$(CC) -o $@ $(BUILD_DIR)/drv/tools/selftest.c.o $(SELFTEST_OBJ) $(shell $(PKG_CONFIG) --libs zlib) -lm

selftest: $(BUILD_DIR)/selftest
	$(BUILD_DIR)/selftest

clean:
	rm -rf $(BUILD_DIR)

.PHONY: all clean selftest
