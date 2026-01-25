# Docker PostgreSQL Setup for Local Development

## Quick Start

### 1. Start PostgreSQL with Docker Compose

```bash
# From project root
docker-compose up -d
```

This will:
- Start PostgreSQL 16 in a Docker container
- Create database: `grainlify`
- Create user: `grainlify`
- Expose port: `5432`

### 2. Database Connection URL

Add this to your `.env` file in the `backend` directory:

```bash
DB_URL=postgresql://grainlify:grainlify_dev_password@localhost:5432/grainlify?sslmode=disable
```

**Full URL breakdown:**
```
postgresql://[user]:[password]@[host]:[port]/[database]?sslmode=disable
```

### 3. Verify Connection

```bash
# Check if container is running
docker ps | grep grainlify-postgres

# Check logs
docker-compose logs postgres

# Connect to database (optional)
docker exec -it grainlify-postgres psql -U grainlify -d grainlify
```

## Environment Variables

### For Backend `.env`:

```bash
# Database
DB_URL=postgresql://grainlify:grainlify_dev_password@localhost:5432/grainlify?sslmode=disable
AUTO_MIGRATE=true

# Other required variables
JWT_SECRET=your-jwt-secret-here
TOKEN_ENC_KEY_B64=your-32-byte-base64-key-here
PUBLIC_BASE_URL=http://localhost:8080
FRONTEND_BASE_URL=http://localhost:5173
```

## Docker Commands

### Start PostgreSQL
```bash
docker-compose up -d
```

### Stop PostgreSQL
```bash
docker-compose down
```

### Stop and Remove Data (⚠️ Deletes all data)
```bash
docker-compose down -v
```

### View Logs
```bash
docker-compose logs -f postgres
```

### Restart PostgreSQL
```bash
docker-compose restart postgres
```

## Custom Configuration

### Change Port

Edit `docker-compose.yml`:
```yaml
ports:
  - "5433:5432"  # Use 5433 on host instead of 5432
```

Then update `DB_URL`:
```bash
DB_URL=postgresql://grainlify:grainlify_dev_password@localhost:5433/grainlify?sslmode=disable
```

### Change Database Name/User/Password

Edit `docker-compose.yml`:
```yaml
environment:
  POSTGRES_USER: your_user
  POSTGRES_PASSWORD: your_password
  POSTGRES_DB: your_database
```

Then update `DB_URL` accordingly.

## Troubleshooting

### Port Already in Use

If port 5432 is already in use:
```bash
# Find what's using the port
lsof -i :5432

# Or change the port in docker-compose.yml
```

### Connection Refused

1. Check if container is running:
   ```bash
   docker ps
   ```

2. Check container logs:
   ```bash
   docker-compose logs postgres
   ```

3. Restart container:
   ```bash
   docker-compose restart postgres
   ```

### Database Doesn't Exist

The database is created automatically when the container starts. If it doesn't exist:

```bash
# Connect to postgres database
docker exec -it grainlify-postgres psql -U grainlify -d postgres

# Create database
CREATE DATABASE grainlify;

# Exit
\q
```

### Reset Database (Fresh Start)

```bash
# Stop and remove container + data
docker-compose down -v

# Start fresh
docker-compose up -d
```

## Alternative: Direct Docker Run

If you prefer not to use docker-compose:

```bash
docker run -d \
  --name grainlify-postgres \
  -e POSTGRES_USER=grainlify \
  -e POSTGRES_PASSWORD=grainlify_dev_password \
  -e POSTGRES_DB=grainlify \
  -p 5432:5432 \
  -v postgres_data:/var/lib/postgresql/data \
  postgres:16-alpine
```

Connection URL remains the same:
```bash
DB_URL=postgresql://grainlify:grainlify_dev_password@localhost:5432/grainlify?sslmode=disable
```

## Production vs Development

**Development (Docker):**
```bash
DB_URL=postgresql://grainlify:grainlify_dev_password@localhost:5432/grainlify?sslmode=disable
```

**Production (Cloud Database):**
```bash
DB_URL=postgresql://user:password@host:5432/database?sslmode=require
```

---

**Note:** The `sslmode=disable` is fine for local development. For production, use `sslmode=require`.















