use std::sync::{Arc, Mutex};

use actix_web::{HttpServer, App, Responder, HttpResponse, get, web::Data};
use dotenv::dotenv;

mod models;
mod tcp_sniffer;
use serde_json::json;
use tcp_sniffer::start_tcp_sniffer;
use models::InverterDataQPIGS;

#[get("/")]
async fn inverter_current_data(data: Data<Arc<Mutex<Option<InverterDataQPIGS>>>>) -> impl Responder {
    let inverter_data = &*data.lock().unwrap();
    if let Some(inverter_data) = inverter_data {
        HttpResponse::Ok().json(inverter_data)
    } else {
        HttpResponse::NotFound().json(json!({"Error": "Resource not found"}))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    println!("Starting sniffer service...");

    let current_inverter_data = start_tcp_sniffer().await.expect("Error starting tcp sniffer");

    println!("Sniffer is up!");

    println!("Starting http server...");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(current_inverter_data.clone()))
            .service(inverter_current_data)
    })
    .bind(("127.0.0.1", 5678))?
    .run()
    .await?;

    println!("Shutting down...");

    Ok(())
}
