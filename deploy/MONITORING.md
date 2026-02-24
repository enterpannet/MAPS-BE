# Monitoring & Logs - Maps Backend + Auth Service

## ตรวจสอบว่า Service ทำงานอยู่หรือไม่

```bash
# สถานะทั้ง 2 service
sudo systemctl status maps-backend maps-auth

# หรือแยกดู
sudo systemctl status maps-backend
sudo systemctl status maps-auth
```

## Health Check (จากเครื่อง server)

```bash
# Backend
curl -s http://127.0.0.1:3001/health
# {"service":"maps-backend","status":"ok"}

# Auth Service
curl -s http://127.0.0.1:3002/health
# {"service":"auth-service","status":"ok"}
```

## ดู Logs (journalctl)

```bash
# Backend - แบบ realtime (กด Ctrl+C ออก)
sudo journalctl -u maps-backend -f

# Auth Service - แบบ realtime
sudo journalctl -u maps-auth -f

# ดูย้อนหลัง 100 บรรทัด
sudo journalctl -u maps-backend -n 100
sudo journalctl -u maps-auth -n 100

# ดู logs วันนี้
sudo journalctl -u maps-backend --since today
sudo journalctl -u maps-auth --since today

# ดูทั้ง 2 service พร้อมกัน
sudo journalctl -u maps-backend -u maps-auth -f
```

## Script ตรวจสอบ Health

มี `deploy/check-health.sh` ใน repo backend:

```bash
# Copy ไปที่ server แล้วรัน
scp backend/deploy/check-health.sh user@server:/opt/
ssh user@server "chmod +x /opt/check-health.sh && /opt/check-health.sh"
```

## Restart Services

```bash
sudo systemctl restart maps-backend
sudo systemctl restart maps-auth
```
