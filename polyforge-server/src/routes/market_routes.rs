use actix_web::web;
use super::handlers;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/market/{symbol}")
            .service(handlers::market_handler::get_live_quote),
    );
}
