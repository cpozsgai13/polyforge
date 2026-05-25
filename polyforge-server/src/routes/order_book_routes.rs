use actix_web::web;
use super::handlers;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/trading/{symbol}")
            .service(handlers::order_book_handler::add_order)
            .service(handlers::order_book_handler::modify_order)
            .service(handlers::order_book_handler::cancel_order)
            .service(handlers::order_book_handler::get_best_bid)
            .service(handlers::order_book_handler::get_best_ask)
            .service(handlers::order_book_handler::get_best_quote),
    );
}
