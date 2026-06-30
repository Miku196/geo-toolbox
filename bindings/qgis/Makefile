# Geo Toolbox QGIS Plugin — Makefile
# Targets: icon, zip, install, clean

PLUGIN_DIR  := geo_toolbox_qgis
ZIP_FILE    := $(PLUGIN_DIR).zip
ICON_FILE   := $(PLUGIN_DIR)/icon.svg

# QGIS profile plugin dirs (platform-dependent)
ifeq ($(OS),Windows_NT)
    QGIS_PLUGIN_DIR := $(APPDATA)/QGIS/QGIS3/profiles/default/python/plugins/$(PLUGIN_DIR)
else
    UNAME_S := $(shell uname -s)
    ifeq ($(UNAME_S),Darwin)
        QGIS_PLUGIN_DIR := $(HOME)/Library/Application Support/QGIS/QGIS3/profiles/default/python/plugins/$(PLUGIN_DIR)
    else
        QGIS_PLUGIN_DIR := $(HOME)/.local/share/QGIS/QGIS3/profiles/default/python/plugins/$(PLUGIN_DIR)
    endif
endif

# ── Default ────────────────────────────────────────────────

.PHONY: all
all: icon zip

# ── Icon generation ────────────────────────────────────────

.PHONY: icon
icon:
	python $(PLUGIN_DIR)/resources.py

# ── Zip for QGIS Plugin Manager ────────────────────────────

.PHONY: zip
zip: icon
	@echo "Packaging $(ZIP_FILE)..."
	@rm -f $(ZIP_FILE)
	@# QGIS expects the plugin folder at the zip root
	cd $(PLUGIN_DIR) && zip -r ../$(ZIP_FILE) . \
		-x "__pycache__/*" \
		-x "*.pyc" \
		-x ".DS_Store" \
		-x "*.swp" \
		-x "tests/*"
	@echo "Done → $(ZIP_FILE)"

# ── Install to QGIS plugin dir ─────────────────────────────

.PHONY: install
install: icon
	@echo "Installing to $(QGIS_PLUGIN_DIR)..."
	@# Use junction / symlink on Windows, symlink on Unix
ifeq ($(OS),Windows_NT)
	@if exist "$(QGIS_PLUGIN_DIR)" rmdir /s /q "$(QGIS_PLUGIN_DIR)"
	mklink /J "$(QGIS_PLUGIN_DIR)" "$(CURDIR)/$(PLUGIN_DIR)"
else
	rm -rf "$(QGIS_PLUGIN_DIR)"
	ln -s "$(CURDIR)/$(PLUGIN_DIR)" "$(QGIS_PLUGIN_DIR)"
endif
	@echo "Done → restart QGIS"

# ── Clean ──────────────────────────────────────────────────

.PHONY: clean
clean:
	rm -f $(ZIP_FILE)
	rm -f $(ICON_FILE)
	rm -rf $(PLUGIN_DIR)/__pycache__
	rm -f $(PLUGIN_DIR)/*.pyc
