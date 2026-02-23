-- อัปเดต checksum ให้ตรงกับไฟล์ migration ปัจจุบัน
-- รัน: psql $DATABASE_URL -f fix_migration_checksums.sql

UPDATE _sqlx_migrations SET checksum = decode('d9673bcafb4543b0240cbc3e732e32bf17d0cf4f7674b8fe6a7a7164506815cd21024fb66b855b57087a9b4dd0981147', 'hex') WHERE version = 1;
UPDATE _sqlx_migrations SET checksum = decode('d2f73d2110aea4cd9164cab0342a66847e030972445ba58ddaa2deb98f1e1066d6c92acfbaaff62451b8d987c3054ded', 'hex') WHERE version = 2;
UPDATE _sqlx_migrations SET checksum = decode('468c31641835ea2b07da74a3644a5d2a89a9dabfb06aec0f79e1a65a2cd56e40ed5bbc3e03f398e4922e5d99845b9e83', 'hex') WHERE version = 3;
UPDATE _sqlx_migrations SET checksum = decode('f1b7d9c0ae570e96bf6fd89815957b3a7bc0b8a41fae7bfb41dfa528d8c5f7d25af300551a09ad8fd51d2835ffc829f3', 'hex') WHERE version = 4;
UPDATE _sqlx_migrations SET checksum = decode('65acb75c0e9ef848651c290c06df7d057e35c103c1047d86d0c4ae8dab17e9a66e4102e3d680bb63d38e34728cbcee50', 'hex') WHERE version = 5;
UPDATE _sqlx_migrations SET checksum = decode('31741ef6811bcce50cca2b40667a1039930b0881d112b1c9a78a0b1e7e6277f0bc296e938756975017f7b44bc8181a8b', 'hex') WHERE version = 6;
