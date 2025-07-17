.PHONY: help dev build up down logs clean test lint format

# Default target
help:
	@echo "Eldoria Hub - Available commands:"
	@echo "  make dev        - Start development environment with hot reloading"
	@echo "  make build      - Build production images"
	@echo "  make up         - Start production containers"
	@echo "  make down       - Stop all containers"
	@echo "  make logs       - View container logs"
	@echo "  make clean      - Clean up containers, volumes, and build artifacts"
	@echo "  make test       - Run all tests"
	@echo "  make lint       - Run linters"
	@echo "  make format     - Format code"

# Development
dev:
	@echo "Starting development environment..."
	docker compose -f docker-compose.dev.yml up 

dev-backend:
	@echo "Starting backend in development mode..."
	cd backend && cargo watch -x run

dev-frontend:
	@echo "Starting frontend in development mode..."
	cd frontend && npm run dev

# Production
build:
	@echo "Building production images..."
	docker compose build --progress=plain --no-cache

up:
	@echo "Starting production containers..."
	docker compose up -d

down:
	@echo "Stopping containers..."
	docker compose down

# Logging
logs:
	docker compose logs -f

logs-backend:
	docker compose logs -f backend

logs-frontend:
	docker compose logs -f frontend

# Testing
test: test-backend test-frontend

test-backend:
	@echo "Running backend tests..."
	cd backend && cargo test

test-frontend:
	@echo "Running frontend tests..."
	cd frontend && npm test

# Linting and formatting
lint: lint-backend lint-frontend

lint-backend:
	@echo "Linting backend..."
	cd backend && cargo clippy -- -D warnings

lint-frontend:
	@echo "Linting frontend..."
	cd frontend && npm run lint

format: format-backend format-frontend

format-backend:
	@echo "Formatting backend..."
	cd backend && cargo fmt

format-frontend:
	@echo "Formatting frontend..."
	cd frontend && npm run format

# Cleanup
clean:
	@echo "Cleaning up..."
	docker compose down -v
	rm -rf backend/target
	rm -rf frontend/node_modules
	rm -rf frontend/dist

# Database operations
db-migrate:
	@echo "Running database migrations..."
	# Add your migration command here

db-seed:
	@echo "Seeding database..."
	# Add your seed command here
