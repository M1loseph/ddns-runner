CREATE TABLE "IpUpdateResults" (
  "id" BIGSERIAL PRIMARY KEY,
  "server_action" VARCHAR(30) NOT NULL,
  "ipv4" INET NOT NULL,
  "ipv6" INET,
  "inserted_at" TIMESTAMP NOT NULL
)