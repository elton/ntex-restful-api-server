-- This file should undo anything in `up.sql`
ALTER TABLE users DROP COLUMN email;
ALTER TABLE users DROP COLUMN avatar;