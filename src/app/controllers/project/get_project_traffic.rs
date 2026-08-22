use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app::models::project_traffic;
use crate::app_errors::AppError;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use chrono::{Datelike, Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use std::collections::HashMap;

#[derive(serde::Deserialize)]
pub struct GetProjectTrafficQuery {
    pub project_id: i32,
}

#[derive(Debug, Serialize)]
pub struct TrafficData {
    pub day: String,
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
    let today = Utc::now().naive_utc().date();
    let seven_days_ago = today - Duration::days(7);

    let rows = project_traffic::Entity::find()
        .filter(project_traffic::Column::ProjectId.eq(project_id))
        .filter(project_traffic::Column::Date.gt(seven_days_ago))
        .order_by_asc(project_traffic::Column::Date)
        .all(pool.as_ref())
        .await
        .map_err(|e| {
            println!("Database error: {}", e);
            AppError::Database(e.to_string())
        })?;

    let traffic_map: HashMap<chrono::NaiveDate, i32> = rows
        .into_iter()
        .map(|r| (r.date, r.request_count))
        .collect();

    let mut result = Vec::new();

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
