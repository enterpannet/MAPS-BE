-- ปั๊มน้ำมันจาก OpenStreetMap (cache)
CREATE TABLE IF NOT EXISTS gas_stations (
    id BIGSERIAL PRIMARY KEY,
    osm_id BIGINT NOT NULL,
    osm_type VARCHAR(10) NOT NULL,
    lat DOUBLE PRECISION NOT NULL,
    lng DOUBLE PRECISION NOT NULL,
    name VARCHAR(255),
    brand VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(osm_type, osm_id)
);

CREATE INDEX IF NOT EXISTS idx_gas_stations_lat_lng ON gas_stations(lat, lng);
