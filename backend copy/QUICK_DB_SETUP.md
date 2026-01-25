# Quick Database Setup

## Using Existing PostgreSQL Container

If you already have a PostgreSQL container running (like `patchwork-postgres`), use this connection URL:

```bash
DB_URL=postgresql://grainlify:grainlify_dev_password@localhost:5432/grainlify?sslmode=disable
```

## Setup Commands (One-time)

Run these commands to create the database and user:

```bash
# Create database
docker exec patchwork-postgres psql -U postgres -c "CREATE DATABASE grainlify;"

# Create user
docker exec patchwork-postgres psql -U postgres -c "CREATE USER grainlify WITH PASSWORD 'grainlify_dev_password';"

# Grant privileges
docker exec patchwork-postgres psql -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE grainlify TO grainlify;"

# Set owner
docker exec patchwork-postgres psql -U postgres -c "ALTER DATABASE grainlify OWNER TO grainlify;"
```

## Alternative: Use Existing Database

If you want to use the existing `patchwork` database:

```bash
DB_URL=postgresql://postgres:postgres@localhost:5432/patchwork?sslmode=disable
```

## Verify Connection

```bash
# Test connection
docker exec patchwork-postgres psql -U grainlify -d grainlify -c "SELECT version();"
```

---

**Note:** The setup commands above have been run automatically. You can now use the connection URL in your `.env` file.















