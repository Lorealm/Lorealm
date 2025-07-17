#!/bin/bash

# Eldoria Hub Development Script
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if .env file exists
if [ ! -f .env ]; then
    print_warning ".env file not found. Creating from .env.example..."
    cp .env.example .env
    print_info "Please update .env with your configuration"
fi

# Parse command line arguments
COMMAND=${1:-all}

case $COMMAND in
    all)
        print_info "Starting all services in development mode..."
        docker-compose -f docker-compose.yml -f docker-compose.dev.yml up
        ;;
    backend)
        print_info "Starting backend in development mode..."
        cd backend && cargo watch -x run
        ;;
    frontend)
        print_info "Starting frontend in development mode..."
        cd frontend && npm run dev
        ;;
    docker)
        print_info "Starting services with Docker..."
        docker-compose -f docker-compose.yml -f docker-compose.dev.yml up
        ;;
    build)
        print_info "Building development containers..."
        docker-compose -f docker-compose.yml -f docker-compose.dev.yml build
        ;;
    logs)
        SERVICE=${2:-}
        if [ -z "$SERVICE" ]; then
            docker-compose -f docker-compose.yml -f docker-compose.dev.yml logs -f
        else
            docker-compose -f docker-compose.yml -f docker-compose.dev.yml logs -f $SERVICE
        fi
        ;;
    shell)
        SERVICE=${2:-backend}
        print_info "Opening shell in $SERVICE container..."
        docker-compose -f docker-compose.yml -f docker-compose.dev.yml exec $SERVICE sh
        ;;
    stop)
        print_info "Stopping all services..."
        docker-compose -f docker-compose.yml -f docker-compose.dev.yml down
        ;;
    clean)
        print_warning "This will remove all containers and volumes. Continue? (y/N)"
        read -r response
        if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
            docker-compose -f docker-compose.yml -f docker-compose.dev.yml down -v
            print_info "Cleanup complete"
        fi
        ;;
    *)
        echo "Usage: $0 {all|backend|frontend|docker|build|logs|shell|stop|clean}"
        echo ""
        echo "Commands:"
        echo "  all       - Start all services (default)"
        echo "  backend   - Start only backend with cargo watch"
        echo "  frontend  - Start only frontend with vite"
        echo "  docker    - Start all services in Docker"
        echo "  build     - Build development containers"
        echo "  logs      - View logs (optionally specify service)"
        echo "  shell     - Open shell in container (default: backend)"
        echo "  stop      - Stop all services"
        echo "  clean     - Remove containers and volumes"
        exit 1
        ;;
esac