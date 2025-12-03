# Rustpad VPS Deployment Script (PowerShell)
# This script deploys Rustpad to Hostinger VPS with Traefik

$ErrorActionPreference = "Stop"

Write-Host "🚀 Deploying Rustpad to VPS..." -ForegroundColor Green

# Configuration
$VPS_HOST = "31.220.59.147"
$VPS_USER = "root"
$DEPLOY_DIR = "/opt/rustpad"
$DOMAIN = "rustpad.thecurtis.cloud"

# Check if .env file exists
if (-not (Test-Path ".env")) {
    Write-Host "⚠️  Warning: .env file not found. Creating from template..." -ForegroundColor Yellow
    Copy-Item ".env.example" ".env"
    Write-Host "❌ Please edit .env file with your OPENROUTER_API_KEY before deploying" -ForegroundColor Red
    exit 1
}

Write-Host "📦 Creating deployment archive..." -ForegroundColor Cyan

# Create list of files to include
$filesToInclude = @(
    "Dockerfile",
    "docker-compose.traefik.yml",
    ".env",
    "rustpad-server",
    "rustpad-wasm",
    "package.json",
    "package-lock.json",
    "src",
    "index.html",
    "tsconfig.json",
    "tsconfig.node.json",
    "vite.config.ts",
    "public"
)

# Use WSL to create tar archive (Windows tar doesn't handle these paths well)
Write-Host "Creating tar archive using WSL..." -ForegroundColor Cyan
wsl bash -c "cd /mnt/d/development/rustpad && tar -czf rustpad-deploy.tar.gz Dockerfile docker-compose.traefik.yml .env rustpad-server rustpad-wasm package.json package-lock.json src index.html tsconfig.json tsconfig.node.json vite.config.ts public"

if (-not (Test-Path "rustpad-deploy.tar.gz")) {
    Write-Host "❌ Failed to create archive" -ForegroundColor Red
    exit 1
}

Write-Host "✅ Archive created successfully" -ForegroundColor Green
Write-Host ""
Write-Host "📤 Now upload the archive to your VPS manually:" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Use WinSCP, FileZilla, or scp to upload rustpad-deploy.tar.gz to /tmp/ on your VPS" -ForegroundColor White
Write-Host "   VPS: $VPS_USER@$VPS_HOST" -ForegroundColor White
Write-Host ""
Write-Host "2. Then SSH into your VPS and run:" -ForegroundColor White
Write-Host ""
Write-Host "   mkdir -p $DEPLOY_DIR" -ForegroundColor Cyan
Write-Host "   cd $DEPLOY_DIR" -ForegroundColor Cyan
Write-Host "   tar -xzf /tmp/rustpad-deploy.tar.gz" -ForegroundColor Cyan
Write-Host "   docker stop rustpad || true" -ForegroundColor Cyan
Write-Host "   docker rm rustpad || true" -ForegroundColor Cyan
Write-Host "   docker build -t rustpad:latest ." -ForegroundColor Cyan
Write-Host "   docker-compose -f docker-compose.traefik.yml up -d" -ForegroundColor Cyan
Write-Host "   docker logs -f rustpad" -ForegroundColor Cyan
Write-Host ""
Write-Host "OR use the Hostinger MCP tools to deploy automatically!" -ForegroundColor Green
