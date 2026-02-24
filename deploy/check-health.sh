#!/bin/bash
# ตรวจสอบ health ของ Backend และ Auth Service
# ใช้บน VPS: chmod +x check-health.sh && ./check-health.sh

echo "=== Maps Services Health Check ==="
echo ""

echo "Backend (port 3001):"
if curl -sf http://127.0.0.1:3001/health > /dev/null; then
  curl -s http://127.0.0.1:3001/health
  echo ""
  echo "  [OK]"
else
  echo "  [FAIL] ไม่สามารถเชื่อมต่อได้"
fi

echo ""
echo "Auth Service (port 3002):"
if curl -sf http://127.0.0.1:3002/health > /dev/null; then
  curl -s http://127.0.0.1:3002/health
  echo ""
  echo "  [OK]"
else
  echo "  [FAIL] ไม่สามารถเชื่อมต่อได้"
fi

echo ""
