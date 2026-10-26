# IAD Quick Start

Get IKAMAI Agent Daemon running in under 5 minutes.

## 1. Install

```bash
# From source
git clone https://github.com/rizoadev/ikamai-agent-daemon.git
cd ikamai-agent-daemon
cargo build --release
cp target/release/iad ~/.local/bin/
```

## 2. Initialize

```bash
iad init
# Creates ~/.iad/{agents,data,logs,run}/
# Creates ~/.iad/iad.toml (edit as needed)
```

## 3. Register your first agent

```bash
cat > ~/.iad/agents/echo.yaml << 'EOF'
name: "echo-agent"
version: "1.0.0"
runtime: "bash"
entrypoint: "/bin/bash"
args: ["-c", "while true; do read line; echo \"Echo: $line\"; done"]
capabilities:
  - "echo"
  - "text-processing"
restart: "on_failure"
max_restarts: 5
EOF

iad agent add ~/.iad/agents/echo.yaml
iad agent list
```

## 4. Start the daemon

```bash
# Foreground (for testing)
iad start --foreground

# Background (production)
iad start
iad status
```

## 5. Check agent status

```bash
iad agent status echo-agent
# 📊 Agent 'echo-agent'
#    Status : running
#    PID    : 12345
#    ...
```

## 6. Submit a task via REST API

```bash
curl -X POST http://127.0.0.1:7700/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{"payload": {"text": "hello"}, "capabilities": ["echo"]}'
```

## 7. Monitor metrics

```bash
curl http://127.0.0.1:7700/metrics
```

## 8. Real-time events (WebSocket)

```bash
websocat ws://127.0.0.1:7700/api/v1/ws?topics=agent,task
```

## 9. Stop

```bash
iad stop
```

---

## Next Steps

- [Agent Development Guide](AGENT_GUIDE.md)
- [Deployment Guide](DEPLOYMENT.md)
- [API Reference](api/openapi.yaml)
