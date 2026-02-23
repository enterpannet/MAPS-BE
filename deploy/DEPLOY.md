# Deploy Backend ไป VPS Ubuntu

## maps-backend.service

ไฟล์อยู่ที่ **`deploy/maps-backend.service`** ใน repo นี้

GitHub Actions จะ copy ไปที่ `/etc/systemd/system/` อัตโนมัติทุกครั้งที่ deploy — **ไม่ต้องสร้างเอง**

## 1. เตรียม VPS (ครั้งแรกเท่านั้น)

```bash
# สร้างโฟลเดอร์
sudo mkdir -p /opt/maps-backend
sudo chown $USER:$USER /opt/maps-backend
```

## 2. สร้าง SSH key สำหรับ deploy

```bash
ssh-keygen -t ed25519 -C "github-deploy" -f ~/.ssh/github_deploy -N ""
cat ~/.ssh/github_deploy.pub >> ~/.ssh/authorized_keys
# Copy private key สำหรับใส่ใน GitHub Secrets
cat ~/.ssh/github_deploy
```

## 3. ตั้งค่า GitHub Secrets

ไปที่ repo → Settings → Secrets and variables → Actions → New repository secret

ดูรายละเอียดที่ `.github/SECRETS.md`

## 4. Deploy

Push ไป main/master จะ deploy อัตโนมัติ

Workflow จะ:
- สร้าง `.env` บน VPS จาก secrets
- Copy binary
- Copy และติดตั้ง `maps-backend.service`
- รีสตาร์ท service
