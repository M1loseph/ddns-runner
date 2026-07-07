CREATE TABLE "Domains" (
  "id" BIGSERIAL PRIMARY KEY,
  "domain_name" VARCHAR(255) NOT NULL UNIQUE
);

INSERT INTO "Domains"("domain_name")
VALUES ('phdwebsite');


ALTER TABLE "IpUpdateResults"
ADD "domain_id" BIGINT REFERENCES "Domains"("id");

UPDATE "IpUpdateResults"
SET "domain_id" = (SELECT "id" FROM "Domains" WHERE "domain_name" = 'phdwebsite');

ALTER TABLE "IpUpdateResults"
ALTER "domain_id" SET NOT NULL;
