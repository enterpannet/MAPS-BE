# Database Migrations

แอป **รัน migration จริง** ตอนสตาร์ท

## วิธีทำงาน

- ตอน build: script จะอ่านทุกไฟล์ `migrations/*.sql` เรียงตามชื่อ แล้ว embed เข้า binary
- ตอนสตาร์ท backend จะเชื่อมต่อ PostgreSQL แล้วรันทีละไฟล์ตามลำดับ
- สถานะว่า migration ไหนรันแล้ว เก็บในตาราง `_app_migrations`
- ถ้า migration ล้มเหลว แอปจะ **ไม่สตาร์ท**

## การเพิ่ม migration ใหม่

1. สร้างไฟล์ `migrations/NNN_name.sql` (NNN เป็นเลขเรียง เช่น 016, 017)
2. เขียน SQL ลงไฟล์ (หลาย statement คั่นด้วย `;\n` ได้)
3. **Build ใหม่** — ไม่ต้องแก้ `migrate.rs` หรือไฟล์อื่น

จากนั้นรันแอป migration จะถูกรันให้อัตโนมัติ (และจะรันแค่ครั้งเดียวต่อแต่ละไฟล์)

## รัน SQL เอง (ถ้าต้องการ)

```bash
psql $DATABASE_URL -f migrations/001_init.sql
# ... ตามลำดับ
```

หรือใช้ client อื่นรันไฟล์ตามลำดับเลข
