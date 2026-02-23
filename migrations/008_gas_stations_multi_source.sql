-- รองรับหลายแหล่งข้อมูลปั๊มน้ำมัน (OSM, NREL, Open Charge Map, Tankerkoenig)
-- ลบ unique เดิมก่อน
ALTER TABLE gas_stations DROP CONSTRAINT IF EXISTS gas_stations_osm_type_osm_id_key;

ALTER TABLE gas_stations ADD COLUMN IF NOT EXISTS source VARCHAR(20) NOT NULL DEFAULT 'osm';
ALTER TABLE gas_stations ADD COLUMN IF NOT EXISTS external_id VARCHAR(100);

-- OSM: external_id = osm_type:osm_id
UPDATE gas_stations SET external_id = osm_type || ':' || osm_id::text WHERE external_id IS NULL;

-- ทำให้ osm_id, osm_type nullable สำหรับแหล่งอื่น
ALTER TABLE gas_stations ALTER COLUMN osm_id DROP NOT NULL;
ALTER TABLE gas_stations ALTER COLUMN osm_type DROP NOT NULL;

-- Unique: source + external_id (ต้องมี external_id สำหรับทุก row)
CREATE UNIQUE INDEX IF NOT EXISTS idx_gas_stations_source_external ON gas_stations(source, external_id);
