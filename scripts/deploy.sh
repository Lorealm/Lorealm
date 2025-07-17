#!/bin/bash

# Eldoria Hub Deployment Script
set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
command -v docker >/dev/null 2>&1 || { print_error "Docker is required but not installed."; exit 1; }
command -v docker-compose >/dev/null 2>&1 || { print_error "Docker Compose is required but not installed."; exit 1; }

# Load environment
if [ ! -f .env ]; then
    print_error ".env file not found!"
    exit 1
fi

# Deployment options
DEPLOY_ENV=${1:-production}
SKIP_BUILD=${2:-false}

print_info "Deploying Eldoria Hub - Environment: $DEPLOY_ENV"

# Build if not skipped
if [ "$SKIP_BUILD" != "skip-build" ]; then
    print_info "Building application..."
    ./scripts/build.sh all production
else
    print_info "Skipping build step..."
fi

# Stop existing containers
print_info "Stopping existing containers..."
docker-compose down

# Backup data (optional)
if [ -d "data" ] && [ "$(ls -A data)" ]; then
    print_warning "Creating backup of user data..."
    tar -czf "backups/data-backup-$(date +%Y%m%d-%H%M%S).tar.gz" data/
fi

# Start new containers
print_info "Starting new containers..."
if [ "$DEPLOY_ENV" = "production" ]; then
    docker-compose up -d
else
    docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d
fi

# Wait for services to be ready
print_info "Waiting for services to be ready..."
sleep 5

# Health check
print_info "Performing health check..."
if curl -f http://localhost:${BACKEND_PORT:-8080}/api/health > /dev/null 2>&1; then
    print_info "Backend is healthy!"
else
    print_error "Backend health check failed!"
    docker-compose logs backend
    exit 1
fi

# Run migrations (if applicable)
# print_info "Running database migrations..."
# docker-compose exec backend ./eldoria-hub migrate

print_info "Deployment completed successfully!"
print_info "Application is running at http://localhost:${FRONTEND_PORT:-80}"