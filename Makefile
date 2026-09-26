PYTHON ?= python3
INSTALL_DESTINATION ?= $(HOME)/Applications

.PHONY: help run format verify verify-full release install

help:
	@echo 'SofDevTool root commands:'
	@echo '  make run          Build and open the Debug app'
	@echo '  make format       Apply Rust formatting'
	@echo '  make verify       Format check, Clippy, tests, Debug app build'
	@echo '  make verify-full  Verify plus Release, Text Diff assets and both bundles'
	@echo '  make release      Package artifacts/SofDevTool.app'
	@echo '  make install      Install Release to INSTALL_DESTINATION (default ~/Applications)'

run:
	$(PYTHON) scripts/rust/package.py --profile debug
	/usr/bin/open 'artifacts/SofDevTool Debug.app'

format:
	scripts/rust/format

verify:
	scripts/rust/verify

verify-full:
	scripts/rust/verify --full
	cd crates/app/src/text_diff/assets-source && npm ci && npm run verify
	$(PYTHON) scripts/rust/package.py --profile debug
	$(PYTHON) scripts/rust/package.py --profile release
	SOFDEVTOOL_EXPECT_UNKNOWN_METADATA=1 cargo test -p sofdevtool-app --lib identity::tests::built_identity_matches_the_cargo_profile_and_version

release:
	$(PYTHON) scripts/rust/package.py --profile release

install:
	$(PYTHON) scripts/rust/install.py --destination '$(INSTALL_DESTINATION)'
