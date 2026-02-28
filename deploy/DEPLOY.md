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

## 5. Reels (วิดีโอสั้น)

- Migration `010_reels.sql` จะรันอัตโนมัติ
- วิดีโอเก็บที่ `./uploads/reels/` (หรือ `UPLOAD_DIR` ใน .env)
- Nginx ต้องมี `client_max_body_size 100M` สำหรับอัปโหลดวิดีโอ

## 6. Posts (โพสต์แบบ Facebook)

- Migration `011_posts.sql`, `012_post_comments.sql` จะรันอัตโนมัติ
- รูปภาพเก็บที่ `./uploads/posts/` (ย่อและบีบอัดอัตโนมัติ)
- ผู้ใช้สามารถคอมเมนต์ได้

## 7. Media Compression

- **รูปภาพ**: ย่อเป็น max 1920px, บีบอัด JPEG quality 88 (คุณภาพสูง)
- **วิดีโอ**: ใช้ ffmpeg บีบอัด H.264 CRF 23, max 1080p — ต้องติดตั้ง `ffmpeg` บน server

## 8. แก้ 502 Bad Gateway

เมื่อ frontend (mapsui.mostdata.site) เรียก `/api/rooms` แล้วได้ **502 Bad Gateway** แปลว่า Nginx ไม่ได้คำตอบจาก backend (mapsapi.mostdata.site หรือ 127.0.0.1:3001)

**บนเครื่องที่รัน backend (mapsapi หรือเครื่องที่รัน maps-backend):**

```bash
# 1. ดูว่า service รันอยู่ไหม
sudo systemctl status maps-backend
# ถ้า inactive: sudo systemctl start maps-backend

# 2. ตรวจ health
curl -s http://127.0.0.1:3001/health
# ควรได้ {"service":"maps-backend","status":"ok"}

# 3. ถ้า frontend อยู่คนละเครื่อง และ /api/ proxy ไป mapsapi — ตรวจบนเครื่อง mapsapi
curl -s https://mapsapi.mostdata.site/health
```

**สาเหตุที่พบบ่อย**

- Backend crash หรือยังไม่สตาร์ท → `sudo systemctl start maps-backend` และดู log: `journalctl -u maps-backend -n 50`
- Migration ล้มเหลวตอนสตาร์ท → ดู log แล้วแก้ DB / รัน migration ให้ผ่าน
- พอร์ต 3001 ถูก firewall หรือ process อื่นใช้อยู่ → ตรวจ `ss -tlnp | grep 3001`

สคริปต์ตรวจทั้ง backend และ auth (ถ้ามี): `./deploy/check-health.sh`
