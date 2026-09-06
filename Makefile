# closure
# ---------------------------------------------------------------------------
# One entry point for the three toolchains this repository uses: Rust for the
# runtime, Node for the web surface, and Python for the validation suite that
# accompanies the manuscript.
#
#   make            list the targets
#   make dev        run server and web together
#   make check      everything CI runs
# ---------------------------------------------------------------------------

.DEFAULT_GOAL := help
SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c

CARGO   ?= cargo
NPM     ?= npm
PYTHON  ?= python
WEB     := web
DOCS    := zuerich/docs/zuerich-common-closure
PSYCHON := zuerich/docs/psychon-phase-mechanics
VALID   := zuerich/validation
PROTO   := zuerich/prototype

# Port the API listens on in development; the web dev server proxies to it.
API_PORT ?= 8080
WEB_PORT ?= 5173

.PHONY: help
help: ## Show this list
	@awk 'BEGIN {FS = ":.*##"; printf "\nclosure\n\n"} \
	  /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2 } \
	  /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) }' $(MAKEFILE_LIST)
	@echo

##@ Setup

.PHONY: setup
setup: setup-rust setup-web ## Install every toolchain

.PHONY: setup-rust
setup-rust: ## Fetch Rust dependencies
	$(CARGO) fetch

.PHONY: setup-web
setup-web: ## Install web dependencies
	cd $(WEB) && $(NPM) install

##@ Develop

.PHONY: dev
dev: ## Run the server and the web surface together
	@echo "api  -> http://localhost:$(API_PORT)"
	@echo "web  -> http://localhost:$(WEB_PORT)"
	@trap 'kill 0' EXIT INT TERM; \
	  CLOSURE_BIND=127.0.0.1:$(API_PORT) $(CARGO) run -p closure-server & \
	  cd $(WEB) && $(NPM) run dev & \
	  wait

.PHONY: dev-server
dev-server: ## Run only the API
	CLOSURE_BIND=127.0.0.1:$(API_PORT) $(CARGO) run -p closure-server

.PHONY: dev-web
dev-web: ## Run only the web surface
	cd $(WEB) && $(NPM) run dev

.PHONY: cli
cli: ## Build the CLI and print its help
	$(CARGO) run -p closure-cli -- --help

##@ Build

.PHONY: build
build: build-rust build-web ## Build everything in release mode

.PHONY: build-rust
build-rust: ## Build the workspace
	$(CARGO) build --workspace --release

.PHONY: build-web
build-web: ## Build the web surface
	cd $(WEB) && $(NPM) run build

##@ Check

.PHONY: check
check: fmt-check lint test ## Everything CI runs

.PHONY: fmt
fmt: ## Format Rust and TypeScript
	$(CARGO) fmt --all
	cd $(WEB) && $(NPM) run fmt

.PHONY: fmt-check
fmt-check: ## Verify formatting without changing files
	$(CARGO) fmt --all -- --check
	cd $(WEB) && $(NPM) run fmt:check

.PHONY: lint
lint: ## Clippy and ESLint, warnings as errors
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	cd $(WEB) && $(NPM) run lint

.PHONY: test
test: ## Rust tests and web typecheck
	$(CARGO) test --workspace
	cd $(WEB) && $(NPM) run typecheck

.PHONY: audit
audit: ## Report vulnerable dependencies
	$(CARGO) audit || echo "install with: cargo install cargo-audit"
	cd $(WEB) && $(NPM) audit --audit-level=high || true

##@ Research artefacts

.PHONY: validate
validate: ## Run the 24-experiment validation suite
	$(PYTHON) $(PROTO)/runtime.py
	$(PYTHON) $(VALID)/run_validation.py

.PHONY: data
data: ## Fetch and normalise the Zurich substrate
	$(PYTHON) $(VALID)/fetch_zurich.py

.PHONY: panels
panels: ## Regenerate the eight publication panels
	$(PYTHON) $(VALID)/make_panels.py

.PHONY: paper
paper: ## Build the manuscript
	cd $(DOCS) && latexmk -pdf -interaction=nonstopmode zuerich-common-closure.tex

.PHONY: paper-psychon
paper-psychon: ## Build the psychon phase mechanics companion
	cd $(PSYCHON) && latexmk -pdf -interaction=nonstopmode psychon-phase-mechanics.tex

.PHONY: papers
papers: paper paper-psychon ## Build every manuscript

.PHONY: paper-clean
paper-clean: ## Remove LaTeX build products
	cd $(DOCS) && latexmk -C
	cd $(PSYCHON) && latexmk -C

##@ Container

.PHONY: docker-build
docker-build: ## Build the server and web images
	docker compose build

.PHONY: docker-up
docker-up: ## Run the stack
	docker compose up --build

.PHONY: docker-down
docker-down: ## Stop the stack
	docker compose down --remove-orphans

##@ Housekeeping

.PHONY: clean
clean: ## Remove build products
	$(CARGO) clean
	rm -rf $(WEB)/dist $(WEB)/node_modules/.vite

.PHONY: doc
doc: ## Open the Rust API documentation
	$(CARGO) doc --workspace --no-deps --open
