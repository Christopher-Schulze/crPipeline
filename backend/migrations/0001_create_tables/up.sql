CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TABLE organizations (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL
);
CREATE TABLE users (
  id UUID PRIMARY KEY,
  org_id UUID REFERENCES organizations(id),
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL
);
