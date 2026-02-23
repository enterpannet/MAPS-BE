-- เพิ่ม updated_at สำหรับ sync job (อัพเดทเฉพาะจุดที่เปลี่ยน)
ALTER TABLE gas_stations ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ;
UPDATE gas_stations SET updated_at = created_at WHERE updated_at IS NULL;
