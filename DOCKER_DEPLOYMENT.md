# Rustpad Docker Deployment Guide

This guide explains how to deploy Rustpad with all features (AI, File Freeze, Admin Panel) using Docker.

## Features Included

- ✅ **Collaborative Text Editing** - Real-time multi-user editing
- ✅ **File Freeze** - Save documents for 30 days with authentication
- ✅ **AI Integration** - OpenRouter-powered AI assistant (with admin-configurable API key)
- ✅ **Admin Panel** - User management and system configuration
- ✅ **Artifact Storage** - Multi-file AI output storage
- ✅ **Persistent Storage** - Docker volumes for user data

## Known Issues

⚠️ **OpenRouter Auto-Router Not Working** - The `openrouter/auto` model currently returns a 401 error. Use specific models like `anthropic/claude-3.5-sonnet` instead.

## Quick Start

### Prerequisites

- Docker 20.10+
- Docker Compose 2.0+

### 1. Clone and Configure

```bash
git clone https://github.com/MichaelDanCurtis/rustpad.git
cd rustpad

# Copy example environment file
cp .env.example .env

# Edit .env and add your OpenRouter API key
nano .env
```

### 2. Build and Run

```bash
# Build the image
docker-compose build

# Start the container
docker-compose up -d

# View logs
docker-compose logs -f
```

### 3. Access Rustpad

Open your browser to: `http://localhost:3030`

## Configuration

### Environment Variables

Edit `.env` file to configure:

```bash
# Required for AI features
OPENROUTER_API_KEY=sk-or-v1-your-key-here

# Optional customization
OPENROUTER_BASE_URL=https://openrouter.ai/api/v1
```

### Docker Compose Settings

Edit `docker-compose.yml` to customize:

```yaml
environment:
  - PORT=3030              # Change port
  - EXPIRY_DAYS=1          # Document cleanup after days
  - ENABLE_AI=true         # Toggle AI features
  - ENABLE_FILE_FREEZE=true
  - ENABLE_ARTIFACTS=true
```

## Admin Setup

### Create Admin User

1. Navigate to `http://localhost:3030`
2. Click "My Files" in footer
3. Click "Register"
4. Fill in credentials
5. **Check "Admin" checkbox** (⚠️ ADMIN WARNING)
6. Click "Register"

### Access Admin Panel

1. Login with admin credentials
2. Click "Admin" button (shield icon) in footer
3. Manage users and configure OpenRouter API key

## API Key Management

You can configure the OpenRouter API key in two ways:

### Option 1: Environment Variable (Docker)
Set `OPENROUTER_API_KEY` in `.env` file before starting container.

### Option 2: Admin Panel (Runtime)
1. Login as admin
2. Open Admin Panel
3. Click "Show Settings"
4. Enter API key and click "Save API Key"

**Note**: Admin panel changes are not persistent across container restarts. Use environment variables for production.

## Data Persistence

Docker volumes store persistent data:

```bash
# Frozen documents and user accounts
rustpad-frozen:/app/frozen_documents

# AI-generated artifacts
rustpad-artifacts:/app/artifacts
```

### Backup Data

```bash
# Create backup
docker run --rm \
  -v rustpad-frozen:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/rustpad-backup.tar.gz /data

# Restore backup
docker run --rm \
  -v rustpad-frozen:/data \
  -v $(pwd):/backup \
  alpine tar xzf /backup/rustpad-backup.tar.gz -C /
```

## Management Commands

```bash
# Start services
docker-compose up -d

# Stop services
docker-compose down

# Restart services
docker-compose restart

# View logs
docker-compose logs -f

# Rebuild after code changes
docker-compose build --no-cache
docker-compose up -d

# Remove all data (⚠️ destructive)
docker-compose down -v
```

## Health Check

Container includes automatic health checking:

```bash
# Check container health
docker ps

# Should show: (healthy) in STATUS column
```

## Production Deployment

### 1. Use Specific Models

Due to auto-router issues, specify exact models:
- `anthropic/claude-3.5-sonnet` (recommended)
- `anthropic/claude-3-haiku` (fast, cheaper)
- `openai/gpt-4-turbo`

### 2. Secure the Instance

```yaml
# Add nginx reverse proxy with SSL
# Example docker-compose snippet:
services:
  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - rustpad
```

### 3. Set Resource Limits

```yaml
services:
  rustpad:
    # ... other config ...
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '0.5'
          memory: 512M
```

### 4. Configure Backups

Set up automated backups:

```bash
# Example cron job
0 2 * * * docker run --rm -v rustpad-frozen:/data -v /backups:/backup alpine tar czf /backup/rustpad-$(date +\%Y\%m\%d).tar.gz /data
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker-compose logs

# Common issues:
# - Port 3030 already in use
# - Invalid API key format
# - Insufficient disk space
```

### AI Features Not Working

1. Check API key is set:
   ```bash
   docker-compose exec rustpad env | grep OPENROUTER
   ```

2. Check logs for API errors:
   ```bash
   docker-compose logs | grep ERROR
   ```

3. Try specific models instead of auto-router

### Cannot Access Admin Panel

1. Ensure user has `is_admin: true` in user file
2. Check browser localStorage for `rustpad_is_admin`
3. Re-register with admin checkbox selected

### Data Not Persisting

```bash
# Check volumes exist
docker volume ls | grep rustpad

# Inspect volume
docker volume inspect rustpad-frozen
```

## Monitoring

### View Resource Usage

```bash
docker stats rustpad
```

### Check Health

```bash
# Health endpoint
curl http://localhost:3030/api/stats

# Should return JSON with server stats
```

## Updating

```bash
# Pull latest changes
git pull origin feature/file-freeze-mcp-integration

# Rebuild and restart
docker-compose down
docker-compose build --no-cache
docker-compose up -d
```

## Support

- GitHub Issues: https://github.com/MichaelDanCurtis/rustpad/issues
- Original Project: https://github.com/ekzhang/rustpad

## License

Same as original Rustpad project (see LICENSE file)
