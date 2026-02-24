-- Reels: short videos (TikTok-style)
CREATE TABLE IF NOT EXISTS reels (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    caption VARCHAR(500) NOT NULL DEFAULT '',
    video_path VARCHAR(500) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reels_user ON reels(user_id);
CREATE INDEX IF NOT EXISTS idx_reels_created ON reels(created_at DESC);
