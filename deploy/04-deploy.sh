#!/bin/bash
# ============================================================
# Deploy Tunelo to VPS
# Run from your local machine (macOS)
#
#   tunelo relay    → /opt/tunelo/bin/tunelo (relay server)
#   website/dist    → /opt/tunelo/website   (tunelo.net)
# ============================================================
set -euo pipefail

VPS="ukvps"
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== 1. Cross-compile tunelo for aarch64-linux ==="
cd "$PROJECT_DIR"
cargo build --release --target aarch64-unknown-linux-musl --bin tunelo

echo ""
echo "=== 2. Build website (tunelo.net) ==="
cd "$PROJECT_DIR/website"
pnpm install
pnpm build

echo ""
echo "=== 3. Upload binary ==="
scp "$PROJECT_DIR/target/aarch64-unknown-linux-musl/release/tunelo" \
    ${VPS}:/tmp/tunelo

echo ""
echo "=== 4. Upload website ==="
scp -r "$PROJECT_DIR/website/dist" ${VPS}:/tmp/tunelo-website

echo ""
echo "=== 5. Upload nginx config and systemd service ==="
scp "$PROJECT_DIR/deploy/tunelo.net.conf" ${VPS}:/tmp/tunelo.net.conf
scp "$PROJECT_DIR/deploy/tunelo-relay.service" ${VPS}:/tmp/tunelo-relay.service

echo ""
echo "=== 6. Install on VPS ==="
ssh ${VPS} bash -s <<'REMOTE'
set -euo pipefail

# Stop and remove old services
sudo systemctl stop tunelo-gateway 2>/dev/null || true
sudo systemctl disable tunelo-gateway 2>/dev/null || true
sudo rm -f /etc/systemd/system/tunelo-gateway.service
sudo rm -f /opt/tunelo/bin/tunelo-gateway
sudo systemctl stop tunelo-relay 2>/dev/null || true
sudo rm -f /opt/tunelo/bin/tunelo-relay

# Install binary
sudo mv /tmp/tunelo /opt/tunelo/bin/tunelo
sudo chmod +x /opt/tunelo/bin/tunelo
sudo chown tunelo:tunelo /opt/tunelo/bin/tunelo

# Install website
sudo mkdir -p /opt/tunelo/website
sudo rm -rf /opt/tunelo/website/*
sudo cp -r /tmp/tunelo-website/* /opt/tunelo/website/
sudo chown -R tunelo:tunelo /opt/tunelo/website/
rm -rf /tmp/tunelo-website

# Install nginx config
sudo mv /tmp/tunelo.net.conf /etc/nginx/sites-available/tunelo.net
sudo ln -sf /etc/nginx/sites-available/tunelo.net /etc/nginx/sites-enabled/tunelo.net
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t
sudo systemctl reload nginx

# Install systemd service
sudo mv /tmp/tunelo-relay.service /etc/systemd/system/tunelo-relay.service
sudo systemctl daemon-reload
sudo systemctl enable tunelo-relay
sudo systemctl restart tunelo-relay

echo ""
echo "=== Service Status ==="
sudo systemctl status tunelo-relay --no-pager -l
REMOTE

echo ""
echo "=== Deploy complete! ==="
echo ""
echo "  https://tunelo.net              → Website"
echo "  *.tunelo.net                    → Tunnel subdomains"
echo "  tunelo http 3000                → Create a tunnel"
