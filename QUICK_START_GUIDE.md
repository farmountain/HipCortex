# HipCortex Quick Start Guide

Welcome to HipCortex! This guide will get you up and running quickly using either Docker/Podman containerization or as a VS Code extension.

## 🚀 Option 1: Docker/Podman Deployment (Recommended)

### Prerequisites
- Docker Desktop or Podman installed
- Git (for cloning the repository)

### Step 1: Clone the Repository
```bash
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex
```

### Step 2: Using Docker

#### Build and Run with Docker
```bash
# Build the Docker image
docker build -t hipcortex:latest .

# Run with default configuration (file-based storage)
docker run -d \
  --name hipcortex \
  -p 3030:3030 \
  -v hipcortex_data:/app/data \
  hipcortex:latest

# Run with PostgreSQL backend
docker run -d \
  --name hipcortex-postgres \
  -p 3030:3030 \
  -e DATABASE_URL="postgresql://postgres:password@postgres:5432/hipcortex" \
  -v hipcortex_data:/app/data \
  --link postgres:postgres \
  hipcortex:latest
```

#### Using Docker Compose (Full Stack)
```bash
# Start all services (HipCortex + PostgreSQL + Redis)
docker-compose up -d

# View logs
docker-compose logs -f hipcortex

# Stop services
docker-compose down
```

### Step 3: Using Podman

#### Build and Run with Podman
```bash
# Build the Podman image
podman build -t hipcortex:latest .

# Run with default configuration
podman run -d \
  --name hipcortex \
  -p 3030:3030 \
  -v hipcortex_data:/app/data \
  hipcortex:latest

# Run with PostgreSQL backend
podman run -d \
  --name hipcortex-postgres \
  -p 3030:3030 \
  -e DATABASE_URL="postgresql://postgres:password@postgres:5432/hipcortex" \
  -v hipcortex_data:/app/data \
  --link postgres:postgres \
  hipcortex:latest
```

#### Using Podman Compose
```bash
# Start all services
podman-compose up -d

# View logs
podman-compose logs -f hipcortex

# Stop services
podman-compose down
```

### Step 4: Verify Installation
```bash
# Check if HipCortex is running
curl http://localhost:3030/health

# Access the web dashboard
# Open http://localhost:3030 in your browser
```

### Environment Variables
Configure HipCortex with these environment variables:

```bash
# Database Configuration
DATABASE_URL=postgresql://user:password@host:5432/database
REDIS_URL=redis://localhost:6379

# Memory Configuration
MEMORY_LIMIT=1GB
CACHE_SIZE=512MB
TEMPORAL_INDEX_SIZE=256MB

# API Configuration
API_PORT=3030
API_KEY=your-secure-api-key
LOG_LEVEL=info

# Feature Flags
ENABLE_POSTGRES=true
ENABLE_TEMPORAL_INDEXING=true
ENABLE_MULTIMODAL=true
ENABLE_WEB_SERVER=true
```

---

## 🔌 Option 2: VS Code Extension Installation

### Method 1: Install from VSIX File (Recommended)

#### Step 1: Download the Extension
```bash
# Clone the repository to get the latest VSIX files
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex/vscode-extension
```

#### Step 2: Install in VS Code
1. Open VS Code
2. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on Mac)
3. Type "Extensions: Install from VSIX..."
4. Select the latest version: `hipcortex-memory-0.1.5.vsix`
5. Click "Install"
6. Reload VS Code when prompted

#### Step 3: Configure the Extension
1. Open VS Code Settings (`Ctrl+,`)
2. Search for "HipCortex"
3. Configure the following settings:
   ```json
   {
     "hipcortex.memory.apiUrl": "http://localhost:3030",
     "hipcortex.memory.apiKey": "your-api-key",
     "hipcortex.memory.autoSync": true,
     "hipcortex.memory.enableContextualMemory": true,
     "hipcortex.memory.memoryLimit": "1GB"
   }
   ```

### Method 2: Build from Source

#### Prerequisites
- Node.js 18+ and npm
- TypeScript
- VS Code Extension Development tools

#### Step 1: Build the Extension
```bash
cd vscode-extension

# Install dependencies
npm install

# Build the extension
npm run compile

# Package the extension
npm run package
```

#### Step 2: Install the Built Extension
```bash
# Install the generated VSIX file
code --install-extension hipcortex-memory-0.1.5.vsix
```

### Using the VS Code Extension

#### Basic Usage
1. **Memory Panel**: Access via View → HipCortex Memory
2. **Quick Capture**: `Ctrl+Shift+M` to capture current context
3. **Search Memory**: `Ctrl+Shift+F` to search your memory bank
4. **Contextual Suggestions**: Automatic suggestions based on current code

#### Commands Available
- `HipCortex: Capture Context` - Save current editor context
- `HipCortex: Search Memory` - Search through captured memories
- `HipCortex: Show Memory Panel` - Open the memory visualization
- `HipCortex: Clear Memory Cache` - Clear local cache
- `HipCortex: Export Memories` - Export memories to file
- `HipCortex: Import Memories` - Import memories from file

#### Memory Types Supported
- **Code Snippets**: Automatically capture useful code patterns
- **Documentation**: Link code to relevant documentation
- **Debugging Context**: Remember debugging sessions and solutions
- **Project Knowledge**: Capture project-specific insights
- **API Usage**: Remember API usage patterns and examples

---

## 🔧 Configuration Options

### Docker Configuration

#### Environment File (.env)
```bash
# Create a .env file in your project directory
DATABASE_URL=postgresql://postgres:password@localhost:5432/hipcortex
REDIS_URL=redis://localhost:6379
API_PORT=3030
LOG_LEVEL=info
ENABLE_WEB_SERVER=true
MEMORY_LIMIT=2GB
```

#### Docker Compose Configuration
```yaml
# docker-compose.yml
version: '3.8'
services:
  hipcortex:
    build: .
    ports:
      - "3030:3030"
    environment:
      - DATABASE_URL=postgresql://postgres:password@postgres:5432/hipcortex
      - REDIS_URL=redis://redis:6379
    depends_on:
      - postgres
      - redis
    volumes:
      - hipcortex_data:/app/data

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=hipcortex
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data

volumes:
  hipcortex_data:
  postgres_data:
  redis_data:
```

### VS Code Extension Configuration

#### Workspace Settings (`.vscode/settings.json`)
```json
{
  "hipcortex.memory.apiUrl": "http://localhost:3030",
  "hipcortex.memory.apiKey": "${env:HIPCORTEX_API_KEY}",
  "hipcortex.memory.autoSync": true,
  "hipcortex.memory.enableContextualMemory": true,
  "hipcortex.memory.memoryTypes": [
    "code",
    "documentation",
    "debugging",
    "api_usage"
  ],
  "hipcortex.memory.syncInterval": 300,
  "hipcortex.memory.maxMemorySize": "1GB",
  "hipcortex.memory.enableNotifications": true
}
```

---

## 🧪 Testing Your Setup

### Test Docker Deployment
```bash
# Health check
curl http://localhost:3030/health

# Test memory storage
curl -X POST http://localhost:3030/api/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Test memory",
    "type": "note",
    "metadata": {"source": "quick_start"}
  }'

# Test memory retrieval
curl http://localhost:3030/api/memories/search?q=test
```

### Test VS Code Extension
1. Open a code file in VS Code
2. Press `Ctrl+Shift+M` to capture context
3. Check the HipCortex Memory panel for the captured memory
4. Use `Ctrl+Shift+F` to search for the captured context

---

## 🚨 Troubleshooting

### Docker Issues
- **Port conflicts**: Change the port mapping `-p 8081:3030`
- **Permission issues**: Use `sudo` or add user to docker group
- **Memory issues**: Increase Docker's memory allocation

### VS Code Extension Issues
- **Extension not loading**: Check VS Code developer tools (Help → Toggle Developer Tools)
- **API connection**: Verify the API URL in settings
- **Authentication**: Ensure API key is correctly configured

### Common Solutions
```bash
# Check logs
docker logs hipcortex

# Restart services
docker-compose restart

# Clean rebuild
docker-compose down
docker-compose build --no-cache
docker-compose up -d
```

---

## 📚 Next Steps

1. **Explore the Web Dashboard**: Visit http://localhost:3030 for the full interface
2. **Read the Documentation**: Check `docs/` folder for detailed guides
3. **Join the Community**: Report issues and contribute on GitHub
4. **Advanced Configuration**: See `PRODUCTION_DEPLOYMENT_GUIDE.md` for production setups

---

## 🔗 Useful Links

- [GitHub Repository](https://github.com/farmountain/HipCortex)
- [Full Documentation](docs/)
- [Production Deployment Guide](PRODUCTION_DEPLOYMENT_GUIDE.md)
- [Environment Setup Guide](Hipcortex_Env_Setup_Guide.md)
- [VS Code Extension Usage Guide](VSCODE_EXTENSION_USAGE_GUIDE.md)

Happy coding with HipCortex! 🚀

