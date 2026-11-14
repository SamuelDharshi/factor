SHELL := /bin/bash
.DEFAULT_GOAL := help
CONTRACTS := vault musdc factor registry

help: ## list targets
	@grep -E '^[a-z-]+:.*##' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  %-10s %s\n", $$1, $$2}'

build: ## build all contracts (cargo near build)
	@for c in $(CONTRACTS); do \
	  if [ -f contracts/$$c/Cargo.toml ]; then \
	    cargo near build non-reproducible-wasm --no-abi --manifest-path contracts/$$c/Cargo.toml || exit 1; \
	  fi; \
	done
	@ls -l target/near/*/*.wasm

test: ## unit + integration tests (local sandbox, no network)
	cargo test --workspace --exclude factor-integration
	cargo test -p factor-integration -- --test-threads=4

unit: ## unit tests only
	cargo test --workspace --exclude factor-integration

lint: ## clippy, warnings as errors
	cargo clippy --workspace --all-targets -- -D warnings

spike: build ## phase-1 spike on testnet (needs .env)
	bash scripts/deploy_spike.sh

deploy: ## deploy musdc, global vault code and factor to testnet (needs .env)
	bash scripts/deploy.sh

dao: ## create the Sputnik DAO (2-of-3 council) and fund it with mUSDC
	bash scripts/dao.sh

demo: ## run the full 8-step story on testnet, print tx links
	bash scripts/demo.sh

web: ## run the UI locally (http://localhost:3000)
	cd web && npm install && npm run dev

all: build test deploy dao demo ## everything

.PHONY: help build test unit lint spike deploy dao demo web all
