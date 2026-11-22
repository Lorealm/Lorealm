# Runs the benchmark against a throwaway server. See README.md.
#
#   make limit                    find the highest load that passes
#   make steps STEPS=100,500      measure fixed load levels
#   make quick                    short run to check everything works
#
# Every run starts a fresh server and mock, and stops them afterwards, even
# on failure or Ctrl-C. Results go to $(RESULTS_DIR) as timestamped JSON.
# Override any variable on the command line, e.g.
#
#   make limit MODEL='mock?ttft_ms=0&tps=0' CRITERIA='--max-added-ttft-ms 100'
#   make steps STEPS=200 DB_CONNECTIONS=32
#   make limit POSTGRES_URL=postgresql://localhost/bench REDIS_URL=redis://localhost

BENCH_DIR   := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
BACKEND     := $(abspath $(BENCH_DIR)/../..)
BIN         := $(BACKEND)/target/release
# Point at another build to compare versions, e.g. a copy made before a change.
SERVER_BIN  ?= $(BIN)/lorealm_server
WORK_DIR    ?= $(BACKEND)/target/bench
RESULTS_DIR ?= $(WORK_DIR)/results

# Runs wipe $(WORK_DIR)/server and `make clean` deletes $(WORK_DIR), so refuse
# anything that would take more with it.
override WORK_DIR := $(abspath $(WORK_DIR))
ifneq ($(filter $(WORK_DIR),/ $(HOME) $(BACKEND) $(abspath $(BACKEND)/..)),)
$(error WORK_DIR=$(WORK_DIR) would delete more than benchmark files; pick a dedicated directory)
endif
ifeq ($(WORK_DIR),)
$(error WORK_DIR is empty)
endif
SERVER_DIR  := $(WORK_DIR)/server

# Where things listen. The server port avoids clashing with a dev server.
SERVER_PORT ?= 15510
MOCK_ADDR   ?= 127.0.0.1:9100
SERVER_URL  := http://127.0.0.1:$(SERVER_PORT)
MOCK_URL    := http://$(MOCK_ADDR)/v1

# The fake model, as mock-llm flags.
MOCK_ARGS   ?= --ttft-ms 600 --tps 60 --min-tokens 200 --max-tokens 600

# Database: SQLite in the run directory unless POSTGRES_URL is set (a Postgres
# database is not wiped between runs). DB_CONNECTIONS overrides the connection
# pool size, which defaults to 5 for SQLite and 10 for Postgres.
POSTGRES_URL   ?=
DB_CONNECTIONS ?=
# Puts a Redis cache in front of the database (either one). Each run uses its
# own key prefix, so nothing is flushed and earlier runs' keys just expire.
REDIS_URL      ?=
REDIS_TTL      ?= 300

# Load settings.
MODE        ?= closed
STEPS       ?= 10,50,100
START       ?= 50
STEP_SECS   ?= 30
WARMUP_SECS ?= 5
MODEL       ?= mock
TOKENIZER   ?= hf:openai-community/gpt2
CRITERIA    ?=
# Anything else for loadgen, e.g. --character-bytes 50000 --think-ms 5000
LOADGEN_ARGS ?=

ADMIN_USER     := admin
ADMIN_PASSWORD := change-me
SERVER_LOG     ?= warn

STAMP := $(shell date +%Y%m%d-%H%M%S)

LOADGEN = $(BIN)/loadgen \
	--server $(SERVER_URL) --mock-url $(MOCK_URL) \
	--admin-user $(ADMIN_USER) --admin-password $(ADMIN_PASSWORD) \
	--mode $(MODE) --model '$(MODEL)' --tokenizer $(TOKENIZER) \
	--step-secs $(STEP_SECS) --warmup-secs $(WARMUP_SECS) \
	--server-pid "$$(cat $(WORK_DIR)/server.pid)" $(LOADGEN_ARGS)

# Starts everything, runs loadgen with $(1), and always shuts down.
define run
	@mkdir -p $(RESULTS_DIR)
	@trap '$(MAKE) --no-print-directory down' EXIT INT TERM; \
	$(MAKE) --no-print-directory up && \
	$(LOADGEN) $(1)
endef

# Starts the server in the background and waits until it answers.
# `exec` keeps the server in the subshell's process, so $$! is its pid.
define start_server
	@(cd $(SERVER_DIR) && \
		CONFIG_PATH=$(SERVER_DIR)/config.yaml \
		LOREALM_ADMIN_USERNAME=$(ADMIN_USER) LOREALM_ADMIN_PASSWORD=$(ADMIN_PASSWORD) \
		LOREALM_POSTGRES_URL='$(POSTGRES_URL)' RUST_LOG=$(SERVER_LOG) \
		exec $(SERVER_BIN)) >> $(WORK_DIR)/server.log 2>&1 & echo $$! > $(WORK_DIR)/server.pid
	@i=0; until curl -fs $(SERVER_URL)/api/v1/health >/dev/null; do \
		i=$$((i + 1)); \
		if [ $$i -ge 60 ]; then \
			echo "server didn't come up; last lines of $(WORK_DIR)/server.log:"; \
			tail -n 20 $(WORK_DIR)/server.log; exit 1; \
		fi; \
		sleep 0.5; \
	done
endef

.PHONY: help build up down limit steps quick clean

help:
	@sed -n '1,13p' $(BENCH_DIR)/Makefile | sed 's/^# \{0,1\}//'
	@echo "Targets: limit, steps, quick, up, down, build, clean"

build:
	cd $(BACKEND) && cargo build --release -p lorealm_server -p bench

## Find the highest load that meets the criteria.
limit: build
	$(call run,--find-limit --start $(START) $(CRITERIA) --json $(RESULTS_DIR)/limit-$(STAMP).json)

## Measure each level in $(STEPS).
steps: build
	$(call run,--steps $(STEPS) --json $(RESULTS_DIR)/steps-$(STAMP).json)

## A short run against an instant model, to check the setup.
quick: build
	$(MAKE) --no-print-directory steps STEPS=5,20 STEP_SECS=5 WARMUP_SECS=1 \
		MODEL='mock?ttft_ms=0&tps=0&tokens=50'

## Start a fresh server and the mock in the background.
up:
	@$(MAKE) --no-print-directory down >/dev/null
	@rm -rf $(SERVER_DIR) && mkdir -p $(SERVER_DIR) && : > $(WORK_DIR)/server.log
	@sed 's/^  port: .*/  port: $(SERVER_PORT)/' $(BENCH_DIR)/server-config.yaml > $(SERVER_DIR)/config.yaml
	$(start_server)
ifneq ($(DB_CONNECTIONS)$(REDIS_URL),)
	@# First-time setup writes the default storage settings, so they can only
	@# be changed after it, followed by a restart.
	@$(MAKE) --no-print-directory down >/dev/null
endif
ifneq ($(DB_CONNECTIONS),)
	@grep -q '^ *max_connections:' $(SERVER_DIR)/config.yaml || \
		{ echo "no max_connections in $(SERVER_DIR)/config.yaml"; exit 1; }
	@sed -i 's/^\( *\)max_connections: .*/\1max_connections: $(DB_CONNECTIONS)/' $(SERVER_DIR)/config.yaml
endif
ifneq ($(REDIS_URL),)
	@grep -q '^  cache: null$$' $(SERVER_DIR)/config.yaml || \
		{ echo "no 'cache: null' to replace in $(SERVER_DIR)/config.yaml"; exit 1; }
	@sed -i 's|^  cache: null$$|  cache:\n    redis_url: $(REDIS_URL)\n    ttl_seconds: $(REDIS_TTL)\n    key_prefix: lorealm-bench-$(STAMP)|' $(SERVER_DIR)/config.yaml
endif
ifneq ($(DB_CONNECTIONS)$(REDIS_URL),)
	$(start_server)
endif
	@$(BIN)/mock-llm --listen $(MOCK_ADDR) --status-secs 0 $(MOCK_ARGS) \
		> $(WORK_DIR)/mock.log 2>&1 & echo $$! > $(WORK_DIR)/mock.pid
	@i=0; until curl -fs http://$(MOCK_ADDR)/stats >/dev/null; do \
		i=$$((i + 1)); \
		if [ $$i -ge 20 ]; then \
			echo "mock didn't come up; see $(WORK_DIR)/mock.log"; exit 1; \
		fi; \
		sleep 0.25; \
	done
	@echo "server $(SERVER_URL) (pid $$(cat $(WORK_DIR)/server.pid)), $(if $(POSTGRES_URL),postgres,sqlite) pool of $$(sed -n 's/^ *max_connections: //p' $(SERVER_DIR)/config.yaml | head -n 1), $(if $(REDIS_URL),redis cache,no cache)"
	@echo "mock $(MOCK_URL) (pid $$(cat $(WORK_DIR)/mock.pid))"

## Stop the server and mock started by `up`.
down:
	@for name in server mock; do \
		pidfile=$(WORK_DIR)/$$name.pid; \
		[ -f $$pidfile ] || continue; \
		pid=$$(cat $$pidfile); \
		if kill $$pid 2>/dev/null; then \
			i=0; while kill -0 $$pid 2>/dev/null && [ $$i -lt 20 ]; do sleep 0.5; i=$$((i + 1)); done; \
			kill -9 $$pid 2>/dev/null || true; \
			echo "stopped $$name"; \
		fi; \
		rm -f $$pidfile; \
	done

## Remove run state and results.
clean: down
	rm -rf $(WORK_DIR)
