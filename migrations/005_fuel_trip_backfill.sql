-- สร้างทริปหลักสำหรับห้องที่มี fuel_records อยู่แล้ว และ assign records ให้ทริปนั้น
INSERT INTO trips (id, room_id, name, created_at)
SELECT gen_random_uuid(), room_id, 'ทริปหลัก', MIN(created_at)
FROM fuel_records
WHERE trip_id IS NULL
GROUP BY room_id;

-- อัปเดต fuel_records ที่ยังไม่มี trip_id ให้ชี้ไปที่ทริปหลักของห้องนั้น
UPDATE fuel_records fr
SET trip_id = (
    SELECT t.id FROM trips t
    WHERE t.room_id = fr.room_id
    ORDER BY t.created_at ASC
    LIMIT 1
)
WHERE fr.trip_id IS NULL;

-- ทำให้ trip_id เป็น NOT NULL (หลัง backfill แล้ว)
ALTER TABLE fuel_records ALTER COLUMN trip_id SET NOT NULL;
