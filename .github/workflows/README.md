# Backend CI

เมื่อแยก backend เป็น repo ของตัวเอง โครงสร้างจะเป็น:

```
backend-repo/
  .github/workflows/backend.yml   ← อยู่ที่ root ของ repo
  src/
  Cargo.toml
  ...
```

Workflow จะรันเมื่อ push/PR ไป main หรือ master
