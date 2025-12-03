# Quick Deployment to VPS

Your deployment archive `rustpad-deploy.tar.gz` (0.14 MB) is ready!

## Current State
- ✅ Archive created with all necessary files
- ✅ Traefik configuration included
- ✅ OpenRouter API key configured
- ✅ Existing rustpad container detected at `/docker/rustpad/`

## Deployment Options

### Option 1: Via SSH (Recommended)

If you have SSH access set up (with keys or password):

```powershell
# 1. Upload the archive
scp rustpad-deploy.tar.gz root@31.220.59.147:/tmp/

# 2. Deploy via SSH
ssh root@31.220.59.147 "
  # Stop existing container
  cd /docker/rustpad && docker-compose down || true
  
  # Create new deployment directory
  mkdir -p /opt/rustpad
  cd /opt/rustpad
  
  # Extract files
  tar -xzf /tmp/rustpad-deploy.tar.gz
  rm /tmp/rustpad-deploy.tar.gz
  
  # Create traefik network if it doesn't exist
  docker network create traefik || true
  
  # Build and deploy
  docker build -t rustpad:latest .
  docker-compose -f docker-compose.traefik.yml up -d
  
  # Show logs
  docker logs rustpad
"
```

### Option 2: Via WinSCP/FileZilla (Manual)

1. **Open WinSCP or FileZilla**
   - Host: `31.220.59.147`
   - Username: `root`
   - Port: `22`

2. **Upload the archive**
   - Upload `rustpad-deploy.tar.gz` to `/tmp/`

3. **Connect via SSH** (PuTTY or any SSH client)
   ```bash
   ssh root@31.220.59.147
   ```

4. **Run deployment commands**
   ```bash
   # Stop existing container
   cd /docker/rustpad && docker-compose down || true
   
   # Create deployment directory
   mkdir -p /opt/rustpad
   cd /opt/rustpad
   
   # Extract files
   tar -xzf /tmp/rustpad-deploy.tar.gz
   rm /tmp/rustpad-deploy.tar.gz
   
   # Create traefik network if needed
   docker network create traefik || true
   
   # Build image
   docker build -t rustpad:latest .
   
   # Deploy
   docker-compose -f docker-compose.traefik.yml up -d
   
   # Check logs
   docker logs -f rustpad
   ```

### Option 3: Via PowerShell (One-liner)

If you have configured SSH keys:

```powershell
# Run this from the rustpad directory
.\deploy-windows.ps1
```

## After Deployment

1. **Check if container is running:**
   ```bash
   docker ps | grep rustpad
   ```

2. **View logs:**
   ```bash
   docker logs -f rustpad
   ```

3. **Test the site:**
   - Open: https://rustpad.thecurtis.cloud
   - Verify HTTPS works
   - Test real-time collaboration
   - Check AI features

4. **Verify Traefik routing:**
   ```bash
   docker logs root-traefik-1 | grep rustpad
   ```

## Troubleshooting

### Container won't start
```bash
docker logs rustpad
```

### Traefik can't route
```bash
# Check if container is on traefik network
docker network inspect traefik | grep rustpad

# If not, add it:
docker network connect traefik rustpad
docker restart rustpad
```

### Build fails
```bash
# Check Docker disk space
docker system df

# Clean up if needed
docker system prune -a
```

## Quick Status Check

```bash
# One command to check everything
docker ps -a | grep rustpad && \
docker logs --tail 50 rustpad && \
curl -I https://rustpad.thecurtis.cloud
```

## Rollback to Previous Version

If something goes wrong:

```bash
cd /docker/rustpad
docker-compose up -d
```

This will restore the previous rustpad instance.
