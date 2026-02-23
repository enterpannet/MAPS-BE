-- Trips: แยกทริปย่อยภายในห้อง (room) เพื่อบันทึกค่าน้ำมันแยกตามทริป
CREATE TABLE IF NOT EXISTS trips (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trips_room ON trips(room_id);

-- เพิ่ม trip_id ใน fuel_records (nullable สำหรับ migration)
ALTER TABLE fuel_records ADD COLUMN IF NOT EXISTS trip_id UUID REFERENCES trips(id) ON DELETE CASCADE;
