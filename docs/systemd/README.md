# HipCortex systemd Service

Run HipCortex as a persistent service on Linux (Ubuntu, Debian, Raspberry Pi OS, Arch).

## Quick install

```bash
# 1. Download binary
sudo curl -L https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-linux-arm64 \
  -o /usr/local/bin/hipcortex && sudo chmod +x /usr/local/bin/hipcortex
# Use hipcortex-linux-amd64 for x86_64 servers

# 2. Create user + data directory
sudo useradd -r -s /bin/false -m -d /var/lib/hipcortex hipcortex
sudo chown hipcortex:hipcortex /var/lib/hipcortex

# 3. Install service
sudo curl -L https://raw.githubusercontent.com/farmountain/HipCortex/main/docs/systemd/hipcortex.service \
  -o /etc/systemd/system/hipcortex.service

# 4. Enable and start
sudo systemctl daemon-reload
sudo systemctl enable hipcortex
sudo systemctl start hipcortex

# 5. Verify
sudo systemctl status hipcortex
curl http://localhost:3030/health  # → ok
```

## Logs

```bash
sudo journalctl -u hipcortex -f
```

## API keys (optional)

Create `/etc/hipcortex/keys.env`:
```
HIPCORTEX_API_KEYS=sk-free-abc:free,sk-pro-xyz:pro
```

Uncomment the `EnvironmentFile` line in the service file, then:
```bash
sudo systemctl daemon-reload && sudo systemctl restart hipcortex
```

## Graceful shutdown

The binary handles `SIGTERM` (used by systemctl stop) and `SIGINT` (Ctrl+C) by flushing the memory store before exit. Safe on sudden power-off when used with the systemd service.
