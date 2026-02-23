-- จุดหมาย จุดพัก จุดแวะ
CREATE TABLE IF NOT EXISTS waypoints (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    name VARCHAR(200) NOT NULL,
    waypoint_type VARCHAR(20) NOT NULL CHECK (waypoint_type IN ('destination', 'rest', 'stopover')),
    lat DOUBLE PRECISION NOT NULL,
    lng DOUBLE PRECISION NOT NULL,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_waypoints_room ON waypoints(room_id);
