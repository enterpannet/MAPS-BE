CREATE TABLE IF NOT EXISTS fuel_records (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    input_mode VARCHAR(10) NOT NULL DEFAULT 'calc',
    distance_km DOUBLE PRECISION,
    km_per_liter DOUBLE PRECISION,
    price_per_liter DOUBLE PRECISION,
    total_cost DOUBLE PRECISION NOT NULL,
    receipt_image TEXT,
    note VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_fuel_records_room ON fuel_records(room_id);
