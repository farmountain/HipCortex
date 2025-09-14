# 🚀 HipCortex Production Deployment Guide
## Complete Setup and Operations Manual

---

## 📋 Prerequisites

### System Requirements
- **Operating System**: Windows 10/11, macOS 10.15+, or Linux (Ubuntu 20.04+)
- **Memory**: Minimum 4GB RAM, Recommended 8GB+ 
- **Storage**: 2GB available disk space
- **Network**: Internet connectivity for dependency downloads

### Software Dependencies
- **Rust**: Latest stable version (1.70+)
- **Cargo**: Comes with Rust installation
- **Git**: For version control and repository cloning
- **VS Code**: For extension deployment (optional)

---

## 🛠️ Installation Steps

### 1. Environment Setup

```bash
# Clone the repository
git clone https://github.com/farmountain/HipCortex.git
cd HipCortex

# Verify Rust installation
rustc --version
cargo --version

# Update Rust to latest stable
rustup update stable
```

### 2. Dependencies Installation

```bash
# Build dependencies and compile project
cargo build --release --features "petgraph_backend,web-server"

# Verify compilation success
cargo test --all --features "petgraph_backend,web-server"
```

### 3. Configuration Setup

#### Environment Variables
Create a `.env` file in the project root:

```env
# Memory Storage Configuration
MEMORY_FILE=memory.jsonl
MEMORY_BACKEND=file  # Options: file, rocksdb, sled

# Web Server Configuration  
SERVER_HOST=127.0.0.1
SERVER_PORT=3030
SERVER_WORKERS=4

# Security Configuration
ENABLE_ENCRYPTION=true
AUTH_SECRET_KEY=your-secure-secret-key-here
RATE_LIMIT_PER_MINUTE=100

# Logging Configuration
LOG_LEVEL=info  # Options: trace, debug, info, warn, error
LOG_FORMAT=json  # Options: json, text

# Performance Configuration
CACHE_SIZE=1000
BATCH_SIZE=100
ASYNC_WORKERS=8
```

#### Production Configuration
For production environments, create `config/production.toml`:

```toml
[server]
host = "0.0.0.0"
port = 3030
workers = 8
max_connections = 1000

[memory]
backend = "rocksdb"
file_path = "/var/lib/hipcortex/memory"
compression = true
backup_enabled = true
backup_interval = 3600  # seconds

[security]
encryption_enabled = true
auth_required = true
rate_limiting = true
cors_enabled = true
allowed_origins = ["https://yourdomain.com"]

[performance]
cache_size = 10000
batch_size = 500
query_timeout = 30
max_memory_mb = 2048

[monitoring]
metrics_enabled = true
health_check_enabled = true
prometheus_endpoint = "/metrics"
```

---

## 🚀 Deployment Options

### Option 1: Direct Binary Deployment

```bash
# Build optimized release binary
cargo build --release --features "petgraph_backend,web-server"

# Copy binary to deployment location
cp target/release/webserver /usr/local/bin/hipcortex-server
cp target/release/cli /usr/local/bin/hipcortex-cli

# Set executable permissions
chmod +x /usr/local/bin/hipcortex-*

# Run server
/usr/local/bin/hipcortex-server
```

### Option 2: Systemd Service (Linux)

Create `/etc/systemd/system/hipcortex.service`:

```ini
[Unit]
Description=HipCortex AI Memory Engine
After=network.target
Wants=network.target

[Service]
Type=simple
User=hipcortex
Group=hipcortex
WorkingDirectory=/opt/hipcortex
Environment=RUST_LOG=info
Environment=MEMORY_FILE=/var/lib/hipcortex/memory.jsonl
ExecStart=/usr/local/bin/hipcortex-server
Restart=always
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl enable hipcortex
sudo systemctl start hipcortex
sudo systemctl status hipcortex
```

### Option 3: Docker Deployment

Create `Dockerfile`:

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --features "petgraph_backend,web-server"

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/webserver /usr/local/bin/
COPY --from=builder /app/target/release/cli /usr/local/bin/

EXPOSE 3030
VOLUME ["/data"]

ENV MEMORY_FILE=/data/memory.jsonl
CMD ["webserver"]
```

Build and run:
```bash
# Build Docker image
docker build -t hipcortex:latest .

# Run container
docker run -d \
  --name hipcortex \
  -p 3030:3030 \
  -v hipcortex-data:/data \
  --restart unless-stopped \
  hipcortex:latest
```

### Option 4: Kubernetes Deployment

Create `k8s-deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: hipcortex
  labels:
    app: hipcortex
spec:
  replicas: 3
  selector:
    matchLabels:
      app: hipcortex
  template:
    metadata:
      labels:
        app: hipcortex
    spec:
      containers:
      - name: hipcortex
        image: hipcortex:latest
        ports:
        - containerPort: 3030
        env:
        - name: MEMORY_FILE
          value: "/data/memory.jsonl"
        - name: SERVER_HOST
          value: "0.0.0.0"
        volumeMounts:
        - name: data-storage
          mountPath: /data
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "1Gi"
            cpu: "500m"
      volumes:
      - name: data-storage
        persistentVolumeClaim:
          claimName: hipcortex-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: hipcortex-service
spec:
  selector:
    app: hipcortex
  ports:
  - protocol: TCP
    port: 80
    targetPort: 3030
  type: LoadBalancer
```

---

## 🔧 VS Code Extension Installation

### Method 1: Direct Installation

```bash
# Navigate to extension directory
cd ui/

# Install dependencies
npm install

# Build extension
npm run build

# Package extension
vsce package

# Install in VS Code
code --install-extension hipcortex-0.1.0.vsix
```

### Method 2: Marketplace Deployment

1. Package extension: `vsce package`
2. Upload to VS Code Marketplace
3. Users install via: `ext install hipcortex`

---

## 📊 Monitoring and Observability

### Health Checks

```bash
# Basic health check
curl http://localhost:3030/health

# Detailed system status
curl http://localhost:3030/status

# Metrics endpoint (if enabled)
curl http://localhost:3030/metrics
```

### Log Management

```bash
# View logs (systemd)
journalctl -u hipcortex -f

# Log rotation configuration
sudo logrotate -d /etc/logrotate.d/hipcortex
```

### Performance Monitoring

```bash
# Memory usage
ps aux | grep hipcortex

# CPU utilization
top -p $(pgrep hipcortex)

# Network connections
netstat -tulpn | grep 3030

# Disk usage
du -sh /var/lib/hipcortex/
```

---

## 🔒 Security Configuration

### SSL/TLS Setup

```bash
# Generate self-signed certificate (development)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Use Let's Encrypt for production
certbot certonly --standalone -d yourdomain.com
```

### Firewall Configuration

```bash
# Allow HTTP/HTTPS traffic
sudo ufw allow 80
sudo ufw allow 443
sudo ufw allow 3030

# Restrict access to specific IPs
sudo ufw allow from 192.168.1.0/24 to any port 3030
```

### Authentication Setup

```bash
# Generate API keys
hipcortex-cli auth generate-key --user admin
hipcortex-cli auth list-keys

# Set up user permissions
hipcortex-cli auth create-user --name developer --role read-write
hipcortex-cli auth create-user --name viewer --role read-only
```

---

## 🛠️ Maintenance and Operations

### Backup Procedures

```bash
# Manual backup
hipcortex-cli backup create --path /backups/hipcortex-$(date +%Y%m%d).tar.gz

# Automated backup script
#!/bin/bash
BACKUP_DIR="/backups"
DATE=$(date +%Y%m%d_%H%M%S)
hipcortex-cli backup create --path "$BACKUP_DIR/hipcortex-$DATE.tar.gz"
find $BACKUP_DIR -name "hipcortex-*.tar.gz" -mtime +7 -delete
```

### Database Maintenance

```bash
# Compact database
hipcortex-cli db compact

# Rebuild indices
hipcortex-cli db reindex

# Vacuum and optimize
hipcortex-cli db vacuum
```

### Updates and Upgrades

```bash
# Check for updates
git pull origin main

# Build new version
cargo build --release --features "petgraph_backend,web-server"

# Graceful restart
sudo systemctl restart hipcortex

# Verify update
curl http://localhost:3030/version
```

---

## 🚨 Troubleshooting

### Common Issues

#### Server Won't Start
```bash
# Check port availability
netstat -tulpn | grep 3030

# Check permissions
ls -la /var/lib/hipcortex/

# Verify configuration
hipcortex-cli config validate
```

#### Memory Issues
```bash
# Check memory usage
free -h
ps aux --sort=-rss | head

# Adjust cache settings
export CACHE_SIZE=500
```

#### Performance Problems
```bash
# Enable debug logging
export RUST_LOG=debug

# Profile performance
cargo bench --features "petgraph_backend,web-server"

# Check database performance
hipcortex-cli db stats
```

### Log Analysis

```bash
# Error patterns
grep -i "error" /var/log/hipcortex.log | tail -20

# Performance metrics
grep "response_time" /var/log/hipcortex.log | awk '{print $NF}'

# Failed requests
grep "status:[45]" /var/log/hipcortex.log
```

---

## 📈 Performance Tuning

### Memory Optimization

```rust
// Production configuration in config/production.toml
[performance]
cache_size = 10000           # Adjust based on available RAM
batch_size = 500             # Optimize for throughput
max_memory_mb = 2048         # Set memory limits
gc_interval = 300            # Garbage collection frequency
```

### Database Tuning

```rust
[database]
max_connections = 100        # Connection pool size
query_timeout = 30           # Query timeout in seconds
cache_size_mb = 512          # Database cache size
wal_enabled = true           # Write-ahead logging
compression = "zstd"         # Compression algorithm
```

### Network Optimization

```rust
[network]
keep_alive = true            # TCP keep-alive
nodelay = true               # Disable Nagle algorithm
buffer_size = 65536          # Socket buffer size
max_concurrent = 1000        # Max concurrent connections
```

---

## 🔄 CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Deploy HipCortex

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        
    - name: Run tests
      run: cargo test --all --features "petgraph_backend,web-server"
      
    - name: Build release
      run: cargo build --release --features "petgraph_backend,web-server"
      
    - name: Deploy to production
      run: |
        scp target/release/webserver production:/usr/local/bin/
        ssh production "systemctl restart hipcortex"
```

---

## 📞 Support and Resources

### Getting Help
- **Documentation**: Complete guides in `docs/` directory
- **Issue Tracking**: GitHub Issues for bug reports
- **Performance**: Benchmark results in `benches/` directory
- **Examples**: Usage examples in `examples/` directory

### Community Resources
- **GitHub Repository**: https://github.com/farmountain/HipCortex
- **API Documentation**: Available at `/docs` endpoint when server is running
- **Performance Benchmarks**: Regular updates in benchmark reports

---

## ✅ Deployment Checklist

- [ ] System requirements verified
- [ ] Dependencies installed
- [ ] Configuration files created
- [ ] Security settings configured
- [ ] SSL certificates installed
- [ ] Firewall rules configured
- [ ] Database initialized
- [ ] Backup procedures setup
- [ ] Monitoring configured
- [ ] Health checks verified
- [ ] Performance tested
- [ ] Documentation reviewed
- [ ] Team training completed

---

*Last Updated: September 13, 2025*  
*HipCortex v0.1.0 - Production Deployment Guide*
