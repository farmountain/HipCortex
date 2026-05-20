# HipCortex Deployment Guide

Three paths: Fly.io managed, Docker self-hosted, binary self-hosted.

---

## Option A — Fly.io (managed, 5 minutes)

```bash
# 1. Install flyctl
curl -L https://fly.io/install.sh | sh

# 2. Login + launch (picks up fly.toml automatically)
fly auth login
fly launch --no-deploy

# 3. Create persistent volume (1 GB, Frankfurt)
fly volumes create hipcortex_data --size 1 --region fra

# 4. Set API keys (format: key:tier,key:tier)
fly secrets set HIPCORTEX_API_KEYS="sk-free-abc123:free,sk-pro-xyz789:pro"

# 5. Deploy
fly deploy

# 6. Verify
curl https://<your-app>.fly.dev/health       # → "ok"
curl https://<your-app>.fly.dev/stats        # → {"total_records":0,...}
curl https://<your-app>.fly.dev/tier \
  -H "X-Api-Key: sk-free-abc123"             # → {"tier":"free",...}
```

**Cost:** ~$1.94/month (shared-cpu-1x, 512MB). Volume: ~$0.15/GB/month.

---

## Option B — Docker self-hosted

```bash
# Build
docker build -t hipcortex:latest .

# Run with persistent storage + API keys
docker run -d \
  --name hipcortex \
  -p 3030:3030 \
  -v hipcortex_data:/app/data \
  -e DATA_DIR=/app/data \
  -e HIPCORTEX_API_KEYS="sk-mykey:team" \
  -e RUST_LOG=info \
  hipcortex:latest

# Verify
curl http://localhost:3030/health
```

### Docker Compose (recommended for production)

```yaml
# docker-compose.prod.yml
version: "3.9"
services:
  hipcortex:
    image: hipcortex:latest
    build: .
    ports: ["3030:3030"]
    volumes:
      - hipcortex_data:/app/data
    environment:
      DATA_DIR: /app/data
      RUST_LOG: info
      # Set via .env file — never commit keys
      HIPCORTEX_API_KEYS: "${HIPCORTEX_API_KEYS}"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3030/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  hipcortex_data:
```

```bash
echo 'HIPCORTEX_API_KEYS=sk-mykey:team' > .env
docker compose -f docker-compose.prod.yml up -d
```

---

## Option C — Binary (edge / embedded)

### Download pre-built binary (no Rust needed)

```bash
# Linux ARM64 (Raspberry Pi 4/5, Jetson, AWS Graviton)
curl -L https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-linux-arm64 \
  -o hipcortex && chmod +x hipcortex && ./hipcortex

# Linux AMD64
curl -L https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-linux-amd64 \
  -o hipcortex && chmod +x hipcortex && ./hipcortex

# macOS ARM64 (M1/M2/M3/M4)
curl -L https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-macos-arm64 \
  -o hipcortex && chmod +x hipcortex && ./hipcortex

# Windows (PowerShell)
Invoke-WebRequest https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-windows-amd64.exe -OutFile hipcortex.exe
.\hipcortex.exe
```

```bash
# Build minimal binary (4 MB, zero external deps)
cargo build --release --bin webserver \
  --no-default-features --features "web-server,petgraph_backend"

# Run
PORT=3030 DATA_DIR=/var/lib/hipcortex \
  HIPCORTEX_API_KEYS="sk-edge:team" \
  ./target/release/webserver
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3030` | Listening port |
| `DATA_DIR` | `.` | Directory for `memory.jsonl` + `audit.log` |
| `HIPCORTEX_API_KEYS` | *(unset = open)* | Comma-sep `key:tier` pairs |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |

### API Key Tiers

| Tier | Write limit | GDPR endpoint | Support |
|------|-------------|--------------|---------|
| `free` | 10,000 records/month | ❌ | community |
| `pro` | 1,000,000 records/month | ✅ | email 48h |
| `team` | unlimited | ✅ | priority 4h |

Generate a key: `openssl rand -hex 24 | sed 's/^/sk-/'`

---

## Connecting the Python SDK

```python
from hipcortex import HipCortexClient

client = HipCortexClient(
    base_url="https://<your-app>.fly.dev",  # or http://localhost:3030
    # api_key="sk-pro-xyz789",              # set if HIPCORTEX_API_KEYS configured
)

client.add_memory(actor="user-42", action="said", target="Hello world")
results = client.search("Hello", limit=5)
print(client.stats())     # GET /stats
print(client.coherence_status())  # GET /coherence/status
```

---

## Health Monitoring

```bash
# All endpoints (unauthenticated)
GET /health          → "ok"
GET /stats           → record counts + metering state
GET /coherence/status → coherence score + invariant checks

# Authenticated (requires X-Api-Key)
GET /tier            → tier info + limits
```

Recommended: wire `/health` into your uptime monitor (Better Uptime, Pingdom, etc.).
