# IAD Deployment Guide

## systemd (recommended for production)

```bash
# Install binary
sudo cp target/release/iad /usr/local/bin/

# Create user
sudo useradd -r -s /bin/false -d /var/lib/iad iad

# Create directories
sudo mkdir -p /var/lib/iad/{agents,data,logs,run}
sudo chown -R iad:iad /var/lib/iad

# Initialize
sudo -u iad HOME=/var/lib/iad iad init

# Install service
sudo cp deploy/iad.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now iad

# Check status
sudo systemctl status iad
journalctl -u iad -f
```

## Docker

```bash
# Build
docker build -t ikamai/iad:latest .

# Run
docker run -d \
  --name iad \
  -p 7700:7700 \
  -v iad-data:/root/.iad \
  ikamai/iad:latest

# Compose
docker compose up -d
```

## Configuration

Edit `~/.iad/iad.toml`:

```toml
[daemon]
log_level = "info"      # debug | info | warn | error
workers   = 4

[api]
rest_enabled = true
rest_bind    = "0.0.0.0:7700"   # Change to 127.0.0.1 for local-only

[health]
check_interval_secs = 10
max_failures        = 3
```

## Security

1. **API Token** — set in environment:
   ```bash
   IAD_API_TOKEN=your-secret-token iad start
   ```

2. **Firewall** — restrict port 7700 to trusted IPs only

3. **Secrets** — use `${secret:NAME}` in agent manifests, store via `iad secret set`

## Multi-Node

Requires Phase 4 gRPC + PostgreSQL backend:
```toml
[storage]
backend = "postgres"
url     = "postgres://iad:pass@db:5432/iad"

[grpc]
enabled = true
bind    = "0.0.0.0:7701"
```
