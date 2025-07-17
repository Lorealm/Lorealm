#!/bin/bash

# Eldoria Hub Cleanup Script
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Parse arguments
CLEAN_LEVEL=${1:-soft}

case $CLEAN_LEVEL in
    soft)
        print_info "Performing soft cleanup..."
        
        # Stop containers
        docker-compose down
        
        # Remove build artifacts
        rm -rf backend/target
        rm -rf frontend/dist
        rm -rf frontend/node_modules/.vite
        
        print_info "Soft cleanup completed!"
        ;;
        
    hard)
        print_warning "This will remove all containers, volumes, and build artifacts!"
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_info "Performing hard cleanup..."
            
            # Stop and remove containers, networks, volumes
            docker-compose down -v
            
            # Remove all build artifacts
            rm -rf backend/target
            rm -rf backend/static
            rm -rf frontend/node_modules
            rm -rf frontend/dist
            
            # Remove Docker images
            docker-compose down --rmi local
            
            print_info "Hard cleanup completed!"
        else
            print_info "Cleanup cancelled."
        fi
        ;;
        
    cache)
        print_info "Cleaning caches..."
        
        # Clean Rust cache
        cd backend && cargo clean && cd ..
        
        # Clean npm cache
        cd frontend && npm cache clean --force && cd ..
        
        # Clean Docker build cache
        docker system prune -f
        
        print_info "Cache cleanup completed!"
        ;;
        
    *)
        echo "Usage: $0 {soft|hard|cache}"
        echo ""
        echo "Levels:"
        echo "  soft  - Stop containers and remove build artifacts (default)"
        echo "  hard  - Remove everything