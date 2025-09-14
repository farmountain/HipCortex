#!/bin/bash

# HipCortex Quick Start Script
# This script helps you get HipCortex running quickly with Docker

set -e

echo "🚀 HipCortex Quick Start Setup"
echo "=============================="

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    echo "   Visit: https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if Docker Compose is available
if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose is not available. Please install Docker Compose."
    exit 1
fi

# Function to use docker compose or docker-compose
docker_compose_cmd() {
    if command -v docker-compose &> /dev/null; then
        docker-compose "$@"
    else
        docker compose "$@"
    fi
}

echo "✅ Docker is installed and ready"

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env file from template..."
    cp .env.example .env
    echo "✅ Created .env file. You can customize it if needed."
fi

# Ask user for deployment type
echo ""
echo "Choose deployment option:"
echo "1) Full stack (HipCortex + PostgreSQL + Redis) - Recommended"
echo "2) Minimal (HipCortex only with file storage)"
echo "3) Development mode (with hot reload)"

read -p "Enter your choice (1-3) [1]: " choice
choice=${choice:-1}

case $choice in
    1)
        echo "🏗️  Building and starting full stack deployment..."
        docker_compose_cmd up -d
        ;;
    2)
        echo "🏗️  Building and starting minimal deployment..."
        docker build -t hipcortex:latest .
        docker run -d \
            --name hipcortex-minimal \
            -p 8080:8080 \
            -v hipcortex_data:/app/data \
            -e ENABLE_POSTGRES=false \
            hipcortex:latest
        ;;
    3)
        echo "🏗️  Starting development environment..."
        docker_compose_cmd -f docker-compose.yml -f docker-compose.dev.yml up -d
        ;;
    *)
        echo "❌ Invalid choice. Please run the script again."
        exit 1
        ;;
esac

echo ""
echo "⏳ Waiting for services to start..."
sleep 10

# Check if HipCortex is running
if curl -f http://localhost:8080/health &> /dev/null; then
    echo "✅ HipCortex is running successfully!"
    echo ""
    echo "🌐 Access points:"
    echo "   • Web Dashboard: http://localhost:8080"
    echo "   • API Health Check: http://localhost:8080/health"
    echo "   • API Documentation: http://localhost:8080/docs"
    echo ""
    echo "🔧 Management commands:"
    echo "   • View logs: docker-compose logs -f hipcortex"
    echo "   • Stop services: docker-compose down"
    echo "   • Restart: docker-compose restart"
    echo ""
    echo "📚 Next steps:"
    echo "   • Read the full documentation in docs/"
    echo "   • Install the VS Code extension from vscode-extension/"
    echo "   • Check the configuration in .env file"
else
    echo "❌ HipCortex failed to start. Check the logs:"
    echo "   docker-compose logs hipcortex"
fi

echo ""
echo "🎉 Setup complete! Happy coding with HipCortex!"
