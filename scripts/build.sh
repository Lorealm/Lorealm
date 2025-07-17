#!/bin/bash

# Eldoria Hub Build Script
set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if .env exists
if [ ! -f .env ]; then
    print_error ".env file not found!"
    exit 1
fi

# Load environment variables
export $(cat .env | grep -v '^#' | xargs)

# Parse arguments
BUILD_TARGET=${1:-all}
ENVIRONMENT=${2:-production}

case $BUILD_TARGET in
    all)
        print_info "Building all components for $ENVIRONMENT..."
        
        # Build backend
        print_info "Building backend..."
        cd backend
        if [ "$ENVIRONMENT" = "production" ]; then
            cargo build --release
        else
            cargo build
        fi
        cd ..
        
        # Build frontend
        print_info "Building frontend..."
        cd frontend
        npm ci
        npm run build
        cd ..
        
        # Copy frontend build to backend static directory
        print_info "Copying frontend build to backend..."
        rm -rf backend/static
        mkdir -p backend/static
        cp -r frontend/dist/* backend/static/
        
        # Build Docker images
        print_info "Building Docker images..."
        docker-compose build
        ;;
        
    backend)
        print_info "Building backend only..."
        cd backend
        if [ "$ENVIRONMENT" = "production" ]; then
            cargo build --release
        else
            cargo build
        fi
        ;;
        
    frontend)
        print_info "Building frontend only..."
        cd frontend
        npm ci
        npm run build
        ;;
        
    docker)
        print_info "Building Docker images only..."
        docker-compose build
        ;;
        
    *)
        echo "Usage: $0 {all|backend|frontend|docker} [production|development]"
        exit 1
        ;;
esac

print_info "Build completed successfully!"