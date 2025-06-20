ALTER TABLE organizations ADD COLUMN api_key UUID UNIQUE NOT NULL DEFAULT uuid_generate_v4();
