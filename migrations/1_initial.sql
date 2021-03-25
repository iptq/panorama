CREATE TABLE IF NOT EXISTS "accounts" (
    "name" TEXT PRIMARY KEY,
    -- hash of the account details, used to check if accounts have changed
    "checksum" TEXT
);

CREATE TABLE IF NOT EXISTS "mail" (
    "id" INTEGER PRIMARY KEY,
    "message_id" TEXT,
    "account" TEXT,
    "folder" TEXT,
    "uidvalidity" INTEGER,
    "uid" INTEGER,
    "filename" TEXT
);
