-- Your SQL goes here
CREATE TABLE "users" (
  id SERIAL PRIMARY KEY,
  name VARCHAR(128) NOT NULL,
  email VARCHAR NOT NULL UNIQUE,
  avatar VARCHAR(128),
  password VARCHAR(128) NOT NULL,
  role VARCHAR(48) NOT NULL DEFAULT 'user',
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  modified_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  deleted_at TIMESTAMP
);
CREATE INDEX "users_email_index" ON "users" (email)