# Database Migrations

Run the SQL manually or use a migration tool:

```bash
# Using psql (Neon/PostgreSQL)
psql $DATABASE_URL -f migrations/001_init.sql
psql $DATABASE_URL -f migrations/002_fuel_records.sql
psql $DATABASE_URL -f migrations/003_fuel_liters.sql
```

Or use any PostgreSQL client to execute the SQL files in order.
