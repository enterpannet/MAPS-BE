//! สร้างโจทย์และข้อมูลศึกษา Rust ด้วย Gemini API

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    /// หัวข้อหรือแนวคิดที่ต้องการ (เช่น "Option and Result", "struct")
    pub topic: String,
    /// full = สร้างทั้ง 4 ขั้น, study = สร้างเฉพาะข้อมูลอ่านศึกษา
    #[serde(default)]
    pub mode: GenerateMode,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum GenerateMode {
    #[default]
    Full,
    Study,
}

#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step1: Option<Step1Payload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step2: Option<Step2Payload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step3: Option<Step3Payload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step4: Option<Step4Payload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Step1Payload {
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Step2Payload {
    pub title: String,
    pub description: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Step3Payload {
    pub title: String,
    pub description: String,
    /// ใช้ ___ แทนช่องว่าง
    pub code_with_blanks: String,
    pub solution: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Step4Payload {
    pub title: String,
    pub task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub solution: String,
}

const GEMINI_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent";

fn build_prompt(topic: &str, mode: GenerateMode) -> String {
    let topic_esc = topic.trim();
    match mode {
        GenerateMode::Study => {
            format!(
                r#"คุณเป็นผู้เชี่ยวชาญภาษา Rust และผู้เขียนสื่อการสอน
งาน: สร้าง "ข้อมูลสำหรับอ่านศึกษา" สำหรับหัวข้อ Rust ต่อไปนี้เท่านั้น

หัวข้อ: {topic_esc}

ส่งคำตอบเป็น JSON เท่านั้น (ไม่มี markdown ไม่มี ```) ในรูปแบบนี้:
{{ "title": "หัวข้อเรื่อง...", "content": "อธิบายแนวคิด สั้นๆ เป็นข้อความ มี bullet ได้ ใช้ \n แยกบรรทัด", "code": "ตัวอย่างโค้ด Rust สั้นๆ ถ้ามี หรือ null ถ้าไม่จำเป็น" }}"#
            )
        }
        GenerateMode::Full => {
            format!(
                r#"คุณเป็นผู้เชี่ยวชาญภาษา Rust และผู้เขียนโจทย์ฝึกหัด
งาน: สร้างโจทย์ฝึกภาษา Rust แบบ 4 ขั้น สำหรับหัวข้อต่อไปนี้

หัวข้อ: {topic_esc}

ส่งคำตอบเป็น JSON เท่านั้น (ไม่มี markdown ไม่มี ```) รูปแบบเดียวกับนี้ทุก field ต้องมี:

{{
  "step1": {{ "title": "หัวข้ออ่านศึกษา", "content": "อธิบายสั้นๆ เป็นข้อความ", "code": "ตัวอย่างโค้ด Rust" }},
  "step2": {{ "title": "พิมพ์ตามตัวอย่าง", "description": "คำอธิบายสั้นๆ", "code": "โค้ดตัวอย่างสั้นๆ ให้ผู้เรียนพิมพ์ตาม" }},
  "step3": {{ "title": "พิมพ์ตามช่องว่าง", "description": "คำอธิบาย", "code_with_blanks": "โค้ดที่ใส่ ___ แทนคำที่ต้องเติม (ใช้ ___ เท่านั้น)", "solution": "โค้ดที่เติมครบแล้ว" }},
  "step4": {{ "title": "พิมพ์เองเลย", "task": "โจทย์ให้เขียนโค้ดจากศูนย์", "hint": "คำใบ้สั้นๆ หรือ null", "solution": "ตัวอย่างคำตอบ" }}
}}

ข้อสำคัญ: โค้ด Rust ต้องเขียนถูก syntax ทุกที่ ใช้ ___ แทนช่องว่างใน step3 เท่านั้น"#
            )
        }
    }
}

async fn call_gemini(config: &Config, prompt: &str) -> Result<String, AppError> {
    let key = config
        .gemini_api_key
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("GEMINI_API_KEY ไม่ได้ตั้งค่า".into()))?;

    let url = format!("{}?key={}", GEMINI_URL, key);
    let body = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": {
            "temperature": 0.4,
            "maxOutputTokens": 4096
        }
    });

    let client = reqwest::Client::new();
    let res = client.post(&url).json(&body).send().await.map_err(|e| {
        tracing::error!("Gemini request failed: {}", e);
        AppError::Internal
    })?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        tracing::error!("Gemini API error {}: {}", status, text);
        return Err(AppError::BadRequest(format!("Gemini API: {}", status)));
    }

    let json: serde_json::Value = res.json().await.map_err(|_| AppError::Internal)?;
    let text = json
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    Ok(text)
}

pub async fn generate(
    State(state): State<crate::AppState>,
    Json(req): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, AppError> {
    if req.topic.trim().is_empty() {
        return Err(AppError::BadRequest("กรุณาระบุหัวข้อ (topic)".into()));
    }

    let prompt = build_prompt(req.topic.trim(), req.mode);
    let raw = call_gemini(&state.config, &prompt).await?;

    let raw_clean = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match req.mode {
        GenerateMode::Study => {
            let step1: Step1Payload = serde_json::from_str(raw_clean).map_err(|e| {
                tracing::warn!("Gemini study JSON parse error: {} raw: {}", e, raw_clean);
                AppError::BadRequest(format!("ไม่สามารถ parse คำตอบจาก Gemini: {}", e))
            })?;
            Ok(Json(GenerateResponse {
                ok: true,
                step1: Some(step1),
                step2: None,
                step3: None,
                step4: None,
                error: None,
            }))
        }
        GenerateMode::Full => {
            let full: FullPayload = serde_json::from_str(raw_clean).map_err(|e| {
                tracing::warn!("Gemini full JSON parse error: {} raw: {}", e, raw_clean);
                AppError::BadRequest(format!("ไม่สามารถ parse คำตอบจาก Gemini: {}", e))
            })?;
            Ok(Json(GenerateResponse {
                ok: true,
                step1: Some(full.step1),
                step2: Some(full.step2),
                step3: Some(full.step3),
                step4: Some(full.step4),
                error: None,
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
struct FullPayload {
    step1: Step1Payload,
    step2: Step2Payload,
    step3: Step3Payload,
    step4: Step4Payload,
}
