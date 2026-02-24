# แก้ /api/auth/health 404 และ Backend ไม่ทำงาน

## ปัญหา

1. **Backend (port 3001)** ไม่ทำงาน → `curl 127.0.0.1:3001` ล้มเหลว
2. **/api/auth/health** คืน 404 → Nginx block สำหรับ HTTPS (443) อาจไม่มี `location /api/auth/`

## แก้ Backend ไม่ทำงาน

```bash
# ติดตั้ง maps-backend.service (ดู MONITORING.md)
sudo cp /path/to/maps-backend.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable maps-backend
sudo systemctl start maps-backend
```

## แก้ /api/auth/health 404

**สาเหตุ:** Server block สำหรับ HTTPS (listen 443) อาจมีแค่ `location /` ที่ proxy ไป 3001 ทำให้ `/api/auth/*` ไปที่ backend แทน auth service

**วิธีแก้:** ตรวจสอบ config ของ Nginx

```bash
# ดู config ที่ใช้อยู่
ls -la /etc/nginx/sites-enabled/

# ดูเนื้อหา (หา server block ที่ listen 443)
sudo cat /etc/nginx/sites-enabled/mapsapi.mostdata.site*

# หรือ
sudo nginx -T | grep -A 200 "listen 443"
```

**ต้องมี** `location /api/auth/` และ `location /api/users/` ใน server block ที่ listen 443 ด้วย:

```nginx
location /api/auth/ {
    proxy_pass http://127.0.0.1:3002;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}

location /api/users/ {
    proxy_pass http://127.0.0.1:3002;
    # ... เหมือนด้านบน
}
```

**ถ้า Certbot สร้างไฟล์แยก** (เช่น `mapsapi.mostdata.site-le-ssl.conf`):
- แก้ไฟล์นั้น หรือ
- ใช้ config ใหม่จาก `backend/nginx/mapsapi.mostdata.site.conf` ที่มีทั้ง HTTP และ HTTPS

```bash
# หลังแก้ config
sudo nginx -t && sudo systemctl reload nginx
```
