# GitHub Actions Secrets (Backend)

ไปที่ **Settings → Secrets and variables → Actions** แล้วเพิ่ม:

## Build & Test

| Secret | Required | ค่า |
|--------|----------|-----|
| **DATABASE_URL** | ✅ | `postgres://maps:PASSWORD@localhost:5432/maps` |
| **REDIS_URL** | ✅ | `redis://localhost:6379` หรือ `redis://:PASSWORD@localhost:6379` |
| **JWT_SECRET** | ✅ | รหัสลับอย่างน้อย 32 ตัว (สำหรับ test) |
| **PORT** | ❌ | `3001` (optional) |

## Deploy

| Secret | Required | ค่า |
|--------|----------|-----|
| **SSH_HOST** | ✅ | IP หรือ domain ของ VPS (เช่น `157.230.240.184`) |
| **SSH_USER** | ✅ | ชื่อ user SSH (เช่น `root`) |
| **SSH_PRIVATE_KEY** | ✅ | เนื้อหา private key ทั้งหมด |
| **DEPLOY_PATH** | ❌ | path บน VPS (default: `/opt/maps-backend`) |

## สร้าง SSH key

```bash
ssh-keygen -t ed25519 -C "github-deploy" -f ~/.ssh/github_deploy -N ""
cat ~/.ssh/github_deploy.pub >> ~/.ssh/authorized_keys
cat ~/.ssh/github_deploy   # copy ทั้งหมดไปใส่ SSH_PRIVATE_KEY
```
