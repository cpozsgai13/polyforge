use actix_web::{get, web, Responder};
use crate::utils::api_response::{self, ApiResponse};

#[get("/hello/{name}")]
pub async fn greet(name: web::Path<String>) -> impl Responder {
    api_response::ApiResponse::new(200, format!("Hello {name}!"))
}

#[get("/test")]
pub async fn test() -> impl Responder {
    println!("Test request handler called");
    api_response::ApiResponse::new(200, "Test".to_string())
}
