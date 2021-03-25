CREATE TABLE IF NOT EXISTS "accounts" (
    "id" INTEGER PRIMARY KEY,
    -- hash of the account details, used to check if accounts have changed
    "checksum" TEXT,
    "name" TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS "mail" (
    "id" INTEGER PRIMARY KEY,
    "account_id" INTEGER,
    "folder" TEXT,
    "uid" INTEGER,

    FOREIGN KEY ("account_id") REFERENCES "accounts" ("id")
);
