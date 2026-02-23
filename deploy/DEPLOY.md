# Deploy Backend ไป VPS Ubuntu

## 1. เตรียม VPS

```bash
# สร้างโฟลเดอร์
sudo mkdir -p /opt/maps-backend
sudo chown $USER:$USER /opt/maps-backend

# Copy .env (สร้างจาก .env.example)
cp .env.example /opt/maps-backend/.env
# แก้ไข .env ให้ถูกต้อง
```

## 2. สร้าง systemd service

```bash
sudo nano /etc/systemd/system/maps-backend.service
```

วางเนื้อหา:

```ini
[Unit]
Description=Maps Backend API
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/maps-backend
ExecStart=/opt/maps-backend/maps-backend
Restart=always
RestartSec=3
EnvironmentFile=/opt/maps-backend/.env

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable maps-backend
```

## 3. สร้าง SSH key สำหรับ deploy

```bash
ssh-keygen -t ed25519 -C "github-deploy" -f ~/.ssh/github_deploy -N ""
cat ~/.ssh/github_deploy.pub >> ~/.ssh/authorized_keys
# Copy private key สำหรับใส่ใน GitHub Secrets
cat ~/.ssh/github_deploy
```

## 4. ตั้งค่า GitHub Secrets

ไปที่ repo → Settings → Secrets and variables → Actions → New repository secret

| Secret | ค่า |
|--------|-----|
| SSH_HOST | IP หรือ domain ของ VPS (เช่น 157.230.240.184) |
| SSH_USER | ชื่อ user (เช่น root) |
| SSH_PRIVATE_KEY | เนื้อหา private key ทั้งหมด |
| DEPLOY_PATH | (optional) path เช่น /opt/maps-backend |

## 5. Deploy

Push ไป main/master จะ deploy อัตโนมัติ
