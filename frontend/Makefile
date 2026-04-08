.PHONY: install clean reset dev-web dev-tauri dev-tauri-nvidia build-web build-tauri build-ui check lint format docker-build docker-up docker-down help

# ─── Setup ───────────────────────────────────────────────────────────

install: ## Install all workspace dependencies
	bun install

clean: ## Remove all build artifacts
	rm -rf apps/web/dist
	rm -rf apps/tauri/dist
	rm -rf packages/ui/dist
	rm -rf packages/ui/.svelte-kit

reset: clean ## Full reset: remove all node_modules and lockfiles, reinstall
	find . -name 'node_modules' -type d -prune -exec rm -rf {} +
	find . -name 'bun.lock' -not -path './bun.lock' -delete
	bun install

# ─── Development ─────────────────────────────────────────────────────

dev-web: ## Run the web app dev server
	@trap 'kill 0' EXIT INT TERM; \
	(cd packages/ui && bunx svelte-package --watch) & \
	sleep 2 && cd apps/web && bun run dev

dev-desktop: ## Run the tauri app in dev mode
	@trap 'kill 0' EXIT INT TERM; \
	(cd packages/ui && bunx svelte-package --watch) & \
	sleep 2 && cd apps/desktop && bun run tauri dev

dev-desktop-nvidia: ## Run the tauri app in dev mode (NVIDIA GPU + Wayland workaround)
	@trap 'kill 0' EXIT INT TERM; \
	(cd packages/ui && bunx svelte-package --watch) & \
	sleep 2 && cd apps/desktop && WEBKIT_DISABLE_DMABUF_RENDERER=1 bun run tauri dev

# ─── Build ───────────────────────────────────────────────────────────

build-ui: ## Build the UI library and generate type definitions
	cd packages/ui && bun run prepack

build-web: ## Build the web app for production
	cd apps/web && bun run build

build-desktop: ## Build the tauri app for production
	cd apps/desktop && bun run tauri build

build-all: build-ui build-web build-tauri ## Build everything

# ─── Quality ─────────────────────────────────────────────────────────

check: ## Run type checking across workspaces
	cd packages/ui && bun run check
	cd apps/tauri && bun run check

lint: ## Run linters
	cd packages/ui && bun run lint

format: ## Format code
	cd packages/ui && bun run format

# ─── Docker ──────────────────────────────────────────────────────────

docker-build: ## Build Docker image(s)
	@echo "TODO: docker build"
	# docker build -t changeit-web -f apps/web/Dockerfile .

docker-up: ## Start Docker containers
	@echo "TODO: docker compose up"
	# docker compose up -d

docker-down: ## Stop Docker containers
	@echo "TODO: docker compose down"
	# docker compose down

docker-logs: ## Tail Docker container logs
	@echo "TODO: docker compose logs"
	# docker compose logs -f

# ─── Help ────────────────────────────────────────────────────────────

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-16s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
