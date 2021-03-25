CREATE TABLE IF NOT EXISTS "accounts" (
    "name" TEXT PRIMARY KEY,
    -- hash of the account details, used to check if accounts have changed
    "checksum" TEXT
);

CREATE TABLE IF NOT EXISTS "mail" (
    "id" INTEGER PRIMARY KEY,
    "internaldate" TEXT,
    "message_id" TEXT,
    "account" TEXT,
    "folder" TEXT,
    "uidvalidity" INTEGER,
    "subject" TEXT,
    "uid" INTEGER,
    "filename" TEXT
);
