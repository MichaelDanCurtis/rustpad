# Rustpad VPS Deployment Guide

This guide explains how to deploy Rustpad on your Hostinger VPS with Traefik reverse proxy.

## Prerequisites

1. **VPS with Traefik**: Your Hostinger VPS should have Traefik already configured with:
   - Docker and Docker Compose installed
   - Traefik network created (`docker network create traefik`)
   - Let's Encrypt certificate resolver configured
   - Websecure entrypoint (port 443)

2. **OpenRouter API Key**: If you want to use AI features, get an API key from [openrouter.ai](https://openrouter.ai/)

3. **DNS Configuration**: Ensure `rustpad.thecurtis.cloud` points to your VPS IP (31.220.59.147)

## Quick Deployment (Automated)

### Step 1: Configure Environment

Copy the example environment file and edit it:

```bash
cp .env.example .env
```

Edit `.env` and set your OpenRouter API key:

```env
OPENROUTER_API_KEY=sk-or-v1-xxxxxxxxxxxxxxxxxxxxx
```

### Step 2: Run Deployment Script

Make the script executable and run it:

```bash
chmod +x deploy-vps.sh
./deploy-vps.sh
```

The script will:
- Create a deployment archive
- Upload it to your VPS
- Stop the existing Rustpad container
- Build the new Docker image
- Deploy with Traefik configuration
- Verify the deployment

### Step 3: Verify

Visit https://rustpad.thecurtis.cloud and verify:
- ✅ Site loads over HTTPS
- ✅ WebSocket connections work
- ✅ AI features are available (if configured)
- ✅ File freeze functionality works

## Manual Deployment

If you prefer to deploy manually:

### 1. Create .env file on VPS

```bash
ssh root@31.220.59.147
mkdir -p /opt/rustpad
cd /opt/rustpad
nano .env
```

Add your configuration:

```env
RUSTPAD_DOMAIN=rustpad.thecurtis.cloud
OPENROUTER_API_KEY=your_key_here
ENABLE_FILE_FREEZE=true
ENABLE_AI=true
ENABLE_ARTIFACTS=true
```

### 2. Copy files to VPS

From your local machine:

```bash
# Create archive (from rustpad directory)
tar -czf rustpad-deploy.tar.gz \
    Dockerfile \
    docker-compose.traefik.yml \
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

# Upload to VPS
scp rustpad-deploy.tar.gz root@31.220.59.147:/opt/rustpad/
```

### 3. Deploy on VPS

```bash
ssh root@31.220.59.147

cd /opt/rustpad

# Extract files
tar -xzf rustpad-deploy.tar.gz

# Stop existing container
docker stop rustpad || true
docker rm rustpad || true

# Build image
docker build -t rustpad:latest .

# Deploy with docker-compose
docker-compose -f docker-compose.traefik.yml up -d

# Check logs
docker logs -f rustpad
```

## Traefik Configuration Explained

The `docker-compose.traefik.yml` includes these Traefik labels:

- **Routing**: Routes `rustpad.thecurtis.cloud` to the container
- **TLS/SSL**: Automatic Let's Encrypt certificates
- **WebSocket Support**: Proper headers for WebSocket connections
- **Security Headers**: X-Frame-Options, X-Content-Type-Options, etc.

## Managing the Deployment

### View Logs

```bash
ssh root@31.220.59.147 'docker logs -f rustpad'
```

### Restart Container

```bash
ssh root@31.220.59.147 'cd /opt/rustpad && docker-compose -f docker-compose.traefik.yml restart'
```

### Update Deployment

Re-run the deployment script:

```bash
./deploy-vps.sh
```

### Stop Container

```bash
ssh root@31.220.59.147 'cd /opt/rustpad && docker-compose -f docker-compose.traefik.yml down'
```

### View Persistent Data

Frozen documents and artifacts are stored in Docker volumes:

```bash
ssh root@31.220.59.147
docker volume ls | grep rustpad
docker volume inspect rustpad_rustpad-frozen
docker volume inspect rustpad_rustpad-artifacts
```

## Troubleshooting

### Container won't start

Check logs:
```bash
ssh root@31.220.59.147 'docker logs rustpad'
```

### Traefik can't route to container

Verify Traefik network:
```bash
ssh root@31.220.59.147 'docker network ls'
ssh root@31.220.59.147 'docker network inspect traefik'
```

The rustpad container should be in the traefik network.

### SSL certificate issues

Check Traefik logs:
```bash
ssh root@31.220.59.147 'docker logs traefik'
```

Ensure your DNS is correctly pointing to the VPS.

### WebSocket connections fail

Verify the WebSocket middleware is applied:
```bash
ssh root@31.220.59.147 'docker inspect rustpad | grep -A 5 traefik'
```

### AI features not working

Check:
1. OPENROUTER_API_KEY is set in .env
2. ENABLE_AI=true in environment
3. OpenRouter API key is valid
4. Check rustpad logs for API errors

## Features

This deployment includes:

- **File Freeze**: 30-day document persistence with authentication
- **AI Integration**: Chat with AI models via OpenRouter
- **Artifacts**: Support for code artifacts and outputs
- **Persistent Storage**: User data survives container restarts
- **Auto-restart**: Container automatically restarts on failure
- **Health Checks**: Automatic health monitoring
- **HTTPS**: Automatic SSL certificates via Let's Encrypt
- **WebSocket Support**: Real-time collaboration

## Security Notes

1. **API Keys**: Keep your OpenRouter API key secure in the .env file
2. **User Data**: Frozen documents are stored in Docker volumes
3. **HTTPS**: All traffic is encrypted via Traefik
4. **Authentication**: File freeze features require user authentication
5. **Admin Controls**: AI access can be controlled per-user by admins

## Support

For issues specific to:
- **Rustpad**: Check the main README.md
- **Traefik**: Check your Traefik configuration
- **VPS**: Contact Hostinger support
