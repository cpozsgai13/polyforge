//! Price-alert REST endpoints + SSE stream.
//!
//! | Method | Path                    | Description                              |
//! |--------|-------------------------|------------------------------------------|
//! | POST   | /api/alerts             | Create a new price alert                 |
//! | GET    | /api/alerts             | List all active (non-triggered) alerts   |
//! | DELETE | /api/alerts/{id}        | Delete an alert                          |
//! | GET    | /api/alerts/events      | SSE stream — fires when alert triggers   |

use actix_web::{delete, get, post, web, HttpRequest, HttpResponse, Responder};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::entities::price_alert::{self, Entity as AlertEntity};
use crate::utils::app_state::AppState;

// ── Request / Response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateAlertRequest {
    pub symbol:    String,
    pub threshold: f64,
    /// "above" or "below"
    pub direction: String,
    pub label:     Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AlertResponse {
    pub id:           i32,
    pub symbol:       String,
    pub threshold:    f64,
    pub direction:    String,
    pub label:        Option<String>,
    pub triggered:    bool,
    /// ISO-8601 UTC string, set when the alert fires.
    pub triggered_at: Option<String>,
}

impl From<price_alert::Model> for AlertResponse {
    fn from(m: price_alert::Model) -> Self {
        Self {
            id:           m.id,
            symbol:       m.symbol,
            threshold:    m.threshold,
            direction:    m.direction,
            label:        m.label,
            triggered:    m.triggered,
            triggered_at: m.triggered_at.map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/alerts — create a new price alert.
#[post("/api/alerts")]
async fn create_alert(
    data: web::Data<AppState>,
    body: web::Json<CreateAlertRequest>,
) -> impl Responder {
    let dir = body.direction.to_lowercase();
    if dir != "above" && dir != "below" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "direction must be \"above\" or \"below\""
        }));
    }

    let now = chrono::Utc::now().naive_utc();
    let new_alert = price_alert::ActiveModel {
        symbol:    Set(body.symbol.to_uppercase()),
        threshold: Set(body.threshold),
        direction: Set(dir),
        label:     Set(body.label.clone()),
        triggered: Set(false),
        created_at: Set(Some(now)),
        ..Default::default()
    };

    match new_alert.insert(&data.db).await {
        Ok(model) => HttpResponse::Created().json(AlertResponse::from(model)),
        Err(e) => {
            log::error!("Failed to create alert: {e}");
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// GET /api/alerts — list all active (non-triggered) alerts.
#[get("/api/alerts")]
async fn list_alerts(data: web::Data<AppState>) -> impl Responder {
    use sea_orm::QueryOrder;
    match AlertEntity::find()
        .order_by_desc(price_alert::Column::Id)
        .all(&data.db)
        .await
    {
        Ok(rows) => {
            let resp: Vec<AlertResponse> = rows.into_iter().map(Into::into).collect();
            HttpResponse::Ok().json(resp)
        }
        Err(e) => {
            log::error!("Failed to list alerts: {e}");
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// DELETE /api/alerts/{id} — delete an alert by id.
#[delete("/api/alerts/{id}")]
async fn delete_alert(
    data: web::Data<AppState>,
    path: web::Path<i32>,
) -> impl Responder {
    let id = path.into_inner();
    match AlertEntity::delete_by_id(id).exec(&data.db).await {
        Ok(res) if res.rows_affected > 0 => HttpResponse::Ok().json(serde_json::json!({ "deleted": id })),
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({ "error": "alert not found" })),
        Err(e) => {
            log::error!("Failed to delete alert {id}: {e}");
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// GET /api/alerts/events — SSE stream.
///
/// Emits `data: <json>\n\n` whenever a price alert fires.  Clients should
/// reconnect on close (standard SSE behaviour).
#[get("/api/alerts/events")]
async fn alert_events(data: web::Data<AppState>, _req: HttpRequest) -> HttpResponse {
    let rx = data.alert_tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|res| {
            match res {
                Ok(event) => {
                    match serde_json::to_string(&event) {
                        Ok(json) => Some(Ok::<_, actix_web::Error>(
                            web::Bytes::from(format!("data: {json}\n\n"))
                        )),
                        Err(e) => {
                            log::warn!("SSE serialisation error: {e}");
                            None
                        }
                    }
                }
                // Lagged — subscriber fell behind; skip.
                Err(_) => None,
            }
        });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create_alert)
       .service(list_alerts)
       .service(delete_alert)
       .service(alert_events);
}
