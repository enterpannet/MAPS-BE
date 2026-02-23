# GitHub Actions Secrets (Backend)

ไปที่ **Settings → Secrets and variables → Actions** แล้วเพิ่ม:

## Build & Test (มี default ไม่ต้องตั้งก็ได้)

| Secret | Required | ค่า |
|--------|----------|-----|
| **DATABASE_URL** | ❌ | default: `postgres://maps:maps@localhost:5432/maps` |
| **REDIS_URL** | ❌ | default: `redis://localhost:6379` |
| **JWT_SECRET** | ❌ | default: `ci-test-secret-key-32-chars-minimum` |
| **PORT** | ❌ | default: `3001` |

## Deploy (.env บน VPS)

Workflow จะสร้าง `.env` บน VPS อัตโนมัติจาก secrets ด้านล่าง  
ถ้าไม่ตั้ง จะใช้ค่า default (เหมาะสำหรับทดสอบ)

| Secret | Required | ค่า |
|--------|----------|-----|
| **DATABASE_URL** | ❌ | สำหรับ production ควรตั้ง |
| **REDIS_URL** | ❌ | สำหรับ production ควรตั้ง |
| **JWT_SECRET** | ❌ | สำหรับ production ต้องตั้งให้แข็งแรง |

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
