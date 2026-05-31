use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app_errors::AppError;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Serialize;
use sqlx::FromRow;
use chrono::{Duration, Utc, Datelike};
use std::collections::HashMap;

#[derive(serde::Deserialize)]
pub struct GetProjectTrafficQuery {
    pub project_id: i32,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TrafficData {
    pub day: String,
    pub value: i32,
}

#[derive(FromRow)]
struct DbTrafficRow {
    pub date: chrono::NaiveDate,
    pub value: i32,
}

pub async fn get_project_traffic_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<GetProjectTrafficQuery>,
) -> Result<HttpResponse, AppError> {
    println!(
        "get_project_traffic_controller called for project_id: {}",
        query.project_id
    );

    let _user_id = req
        .extensions()
        .get::<AuthMiddleware>()
        .ok_or_else(|| {
            println!("Error: AuthMiddleware not found in request extensions");
            AppError::InternalServerError
        })?
        .user_id;

    let project_id = query.project_id;

    let query_str = r#"
        SELECT 
            date,
            request_count as value
        FROM project_traffic
        WHERE project_id = $1 AND date > CURRENT_DATE - INTERVAL '7 days'
        ORDER BY date ASC
    "#;

    let rows: Vec<DbTrafficRow> = sqlx::query_as(query_str)
        .bind(project_id)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| {
            println!("Database error: {}", e);
            AppError::Database(e.to_string())
        })?;

    let traffic_map: HashMap<chrono::NaiveDate, i32> = rows
        .into_iter()
        .map(|r| (r.date, r.value))
        .collect();

    let mut result = Vec::new();
    let today = Utc::now().naive_utc().date();

    for i in (0..7).rev() {
        let date = today - Duration::days(i as i64);
        let day_name = match date.weekday() {
            chrono::Weekday::Mon => "Mon",
            chrono::Weekday::Tue => "Tue",
            chrono::Weekday::Wed => "Wed",
            chrono::Weekday::Thu => "Thu",
            chrono::Weekday::Fri => "Fri",
            chrono::Weekday::Sat => "Sat",
            chrono::Weekday::Sun => "Sun",
        };

        let value = *traffic_map.get(&date).unwrap_or(&0);
        result.push(TrafficData {
            day: day_name.to_string(),
            value,
        });
    }

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Successfully fetched project traffic data".to_string(),
        data: Some(result),
    }))
}
