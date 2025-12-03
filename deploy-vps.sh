#!/bin/bash

# Rustpad VPS Deployment Script
# This script deploys Rustpad to Hostinger VPS with Traefik

set -e

echo "🚀 Deploying Rustpad to VPS..."

# Configuration
VPS_HOST="31.220.59.147"
VPS_USER="root"
DEPLOY_DIR="/opt/rustpad"
DOMAIN="rustpad.thecurtis.cloud"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if .env file exists
if [ ! -f ".env" ]; then
    echo -e "${YELLOW}⚠️  Warning: .env file not found. Creating from template...${NC}"
    cp .env.example .env
    echo -e "${RED}❌ Please edit .env file with your OPENROUTER_API_KEY before deploying${NC}"
    exit 1
fi

echo "📦 Creating deployment archive..."
tar -czf rustpad-deploy.tar.gz \
    Dockerfile \
    docker-compose.traefik.yml \
    .env \
    rustpad-server/ \
    rustpad-wasm/ \
    package.json \
    package-lock.json \
    src/ \
    index.html \
    tsconfig.json \
    tsconfig.node.json \
    vite.config.ts \
    public/

echo "📤 Uploading to VPS..."
scp rustpad-deploy.tar.gz ${VPS_USER}@${VPS_HOST}:/tmp/

echo "🔧 Deploying on VPS..."
ssh ${VPS_USER}@${VPS_HOST} << 'ENDSSH'
set -e

DEPLOY_DIR="/opt/rustpad"
DOMAIN="rustpad.thecurtis.cloud"

# Create deployment directory
sudo mkdir -p ${DEPLOY_DIR}
cd ${DEPLOY_DIR}

# Stop existing container if running
echo "🛑 Stopping existing Rustpad container..."
if docker ps -a | grep -q rustpad; then
    docker stop rustpad || true
    docker rm rustpad || true
fi

# Extract new version
echo "📦 Extracting deployment files..."
sudo tar -xzf /tmp/rustpad-deploy.tar.gz -C ${DEPLOY_DIR}
rm /tmp/rustpad-deploy.tar.gz

# Build the image
echo "🏗️  Building Docker image..."
docker build -t rustpad:latest .

# Deploy with docker-compose
echo "🚀 Starting Rustpad with Traefik..."
docker-compose -f docker-compose.traefik.yml up -d

# Wait for container to be healthy
echo "⏳ Waiting for container to be healthy..."
sleep 10

# Check status
if docker ps | grep -q rustpad; then
    echo "✅ Rustpad deployed successfully!"
    echo "🌐 Access at: https://${DOMAIN}"
    docker ps | grep rustpad
else
    echo "❌ Deployment failed. Checking logs..."
    docker logs rustpad
    exit 1
fi

ENDSSH

echo -e "${GREEN}✅ Deployment complete!${NC}"
echo -e "${GREEN}🌐 Rustpad is now running at: https://${DOMAIN}${NC}"

# Cleanup
rm rustpad-deploy.tar.gz

echo ""
echo "To view logs: ssh ${VPS_USER}@${VPS_HOST} 'docker logs -f rustpad'"
echo "To restart: ssh ${VPS_USER}@${VPS_HOST} 'cd ${DEPLOY_DIR} && docker-compose -f docker-compose.traefik.yml restart'"
