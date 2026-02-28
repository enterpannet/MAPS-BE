-- Add speed (km/h) and heading (degrees 0-360) to locations for navigation sharing
ALTER TABLE locations ADD COLUMN IF NOT EXISTS speed REAL;
ALTER TABLE locations ADD COLUMN IF NOT EXISTS heading REAL;
