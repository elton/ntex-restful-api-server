-- This file should undo anything in `up.sql`
ALTER TABLE users DROP COLUMN role;
ALTER TABLE users DROP COLUMN password;