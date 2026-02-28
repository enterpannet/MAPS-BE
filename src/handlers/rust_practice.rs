//! สร้างโจทย์และข้อมูลศึกษา Rust ด้วย Gemini API + บันทึก/โหลดจาก DB

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::rust_practice_topic;
use crate::AppState;

// ─── Gemini generate ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    pub topic: String,
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

const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const GEMINI_MODEL_DEFAULT: &str = "gemini-2.0-flash";

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

async fn call_gemini(
    config: &crate::config::Config,
    prompt: &str,
    api_key_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<String, AppError> {
    let key = api_key_override
        .filter(|s| !s.trim().is_empty())
        .or_else(|| config.gemini_api_key.as_deref())
        .ok_or_else(|| {
            AppError::BadRequest(
                "กรุณาตั้งค่า GEMINI_API_KEY ในเซิร์ฟเวอร์ หรือกรอก API Key ในหน้าตั้งค่า".into(),
            )
        })?;

    let model = model_override
        .filter(|s| !s.trim().is_empty())
        .or_else(|| config.gemini_model.as_deref())
        .unwrap_or(GEMINI_MODEL_DEFAULT);
    let url = format!("{}/{}:generateContent?key={}", GEMINI_BASE, model, key);
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
        let msg = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|j| {
                j.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| format!("Gemini API: {}", status));
        return Err(AppError::BadRequest(msg));
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
    State(state): State<AppState>,
    headers: HeaderMap,
    AuthUser(_auth): AuthUser,
    Json(req): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, AppError> {
    if req.topic.trim().is_empty() {
        return Err(AppError::BadRequest("กรุณาระบุหัวข้อ (topic)".into()));
    }

    let api_key_override = headers
        .get("x-gemini-api-key")
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok());
    let model_override = headers
        .get("x-gemini-model")
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok());

    let prompt = build_prompt(req.topic.trim(), req.mode);
    let raw = call_gemini(&state.config, &prompt, api_key_override, model_override).await?;

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

// ─── Topic CRUD ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TopicResponse {
    pub id: String,
    pub title: String,
    pub s1_title: String,
    pub s1_content: String,
    pub s1_code: Option<String>,
    pub s2_title: String,
    pub s2_description: String,
    pub s2_code: String,
    pub s3_title: String,
    pub s3_description: String,
    pub s3_code_with_blanks: String,
    pub s3_solution: String,
    pub s4_title: String,
    pub s4_task: String,
    pub s4_hint: Option<String>,
    pub s4_solution: String,
    pub created_at: String,
}

fn model_to_response(m: rust_practice_topic::Model) -> TopicResponse {
    TopicResponse {
        id: m.id.to_string(),
        title: m.title,
        s1_title: m.s1_title,
        s1_content: m.s1_content,
        s1_code: m.s1_code,
        s2_title: m.s2_title,
        s2_description: m.s2_description,
        s2_code: m.s2_code,
        s3_title: m.s3_title,
        s3_description: m.s3_description,
        s3_code_with_blanks: m.s3_code_with_blanks,
        s3_solution: m.s3_solution,
        s4_title: m.s4_title,
        s4_task: m.s4_task,
        s4_hint: m.s4_hint,
        s4_solution: m.s4_solution,
        created_at: m.created_at.to_rfc3339(),
    }
}

/// GET /api/rust-practice/topics — โหลดหัวข้อทั้งหมดจาก DB
pub async fn list_topics(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
) -> Result<Json<Vec<TopicResponse>>, AppError> {
    let topics = rust_practice_topic::Entity::find()
        .order_by_asc(rust_practice_topic::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(topics.into_iter().map(model_to_response).collect()))
}

#[derive(Debug, Deserialize)]
pub struct SaveTopicRequest {
    pub title: String,
    pub step1: Step1Payload,
    pub step2: Step2Payload,
    pub step3: Step3Payload,
    pub step4: Step4Payload,
}

/// POST /api/rust-practice/topics — บันทึกหัวข้อที่สร้างจาก Gemini ลง DB (ผู้ใช้ที่ล็อกอินแล้ว)
pub async fn save_topic(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
    Json(req): Json<SaveTopicRequest>,
) -> Result<Json<TopicResponse>, AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title ต้องไม่ว่าง".into()));
    }

    let now = chrono::Utc::now();
    let record = rust_practice_topic::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        title: ActiveValue::Set(req.title.trim().to_string()),
        s1_title: ActiveValue::Set(req.step1.title),
        s1_content: ActiveValue::Set(req.step1.content),
        s1_code: ActiveValue::Set(req.step1.code),
        s2_title: ActiveValue::Set(req.step2.title),
        s2_description: ActiveValue::Set(req.step2.description),
        s2_code: ActiveValue::Set(req.step2.code),
        s3_title: ActiveValue::Set(req.step3.title),
        s3_description: ActiveValue::Set(req.step3.description),
        s3_code_with_blanks: ActiveValue::Set(req.step3.code_with_blanks),
        s3_solution: ActiveValue::Set(req.step3.solution),
        s4_title: ActiveValue::Set(req.step4.title),
        s4_task: ActiveValue::Set(req.step4.task),
        s4_hint: ActiveValue::Set(req.step4.hint),
        s4_solution: ActiveValue::Set(req.step4.solution),
        created_at: ActiveValue::Set(now.into()),
    };

    let saved = record
        .insert(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(model_to_response(saved)))
}

/// DELETE /api/rust-practice/topics/:id — ลบหัวข้อ (admin)
pub async fn delete_topic(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(topic_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if auth.role != "admin" {
        return Err(AppError::Forbidden);
    }

    let id =
        Uuid::parse_str(&topic_id).map_err(|_| AppError::BadRequest("Invalid topic id".into()))?;

    rust_practice_topic::Entity::delete_by_id(id)
        .exec(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
