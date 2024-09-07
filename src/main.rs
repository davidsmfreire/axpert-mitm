use std::sync::{Arc, Mutex};

use actix_web::{get, web::Data, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use dotenv::dotenv;

mod models;
mod tcp_sniffer;
use models::{InverterDataQPIGS, InverterStatusQPIGS};
use serde_json::json;
use tcp_sniffer::start_tcp_sniffer;
use watchpower_api::{WatchPowerAPI, WatchPowerLastData};

#[derive(Debug, Clone)]
struct AppData {
    sniffer_data: Arc<Mutex<Option<InverterDataQPIGS>>>,
    api_data: Option<WatchPowerLastData>,
    api_client: WatchPowerAPI,
}

fn inverter_data_from_watch_power_last_data(input: &WatchPowerLastData) -> InverterDataQPIGS {
    InverterDataQPIGS {
        timestamp: input.timestamp,
        grid_voltage: input.main.grid_voltage,
        grid_frequency: input.main.grid_frequency,
        ac_output_voltage: input.main.ac_output_voltage,
        ac_output_frequency: input.main.ac_output_frequency,
        ac_output_apparent_power: input.main.ac_output_apparent_power as u16,
        ac_output_active_power: input.main.ac_output_active_power as f32,
        ac_output_load_percent: input.main.output_load_percent as f32,
        bus_voltage: -999.0,
        bat_voltage: input.main.battery_voltage,
        bat_charge_current: input.main.battery_charging_current,
        bat_capacity: input.main.battery_capacity as f32,
        heat_sink_temp: -999.0,
        pv_current: input.pv.pv_input_current,
        pv_voltage: input.main.pv_input_voltage,
        bat_voltage_from_scc: -999.0,
        bat_discharge_current: input.main.battery_discharge_current,
        bat_volt_offset: -999.0,
        eeprom_version: 0,
        pv_power: input.main.pv_input_power as u16,
        status: InverterStatusQPIGS {
            add_sbu_priority_version: false,
            config_changed: false,
            scc_firmware_updates: false,
            load_on: false,
            bat_volt_to_steady: false,
            charging: false,
            charging_scc: false,
            charging_ac: false,
            charging_to_floating_point: false,
            switch_on: false,
            reserved: false,
        },
    }
}

impl AppData {
    fn update_api_data(&mut self) -> Option<WatchPowerLastData> {
        match self.api_client.login(
            &std::env::var("WATCHPOWER_API_USERNAME").expect("WATCHPOWER_API_USERNAME must be set"),
            &std::env::var("WATCHPOWER_API_PASSWORD").expect("WATCHPOWER_API_PASSWORD must be set"),
        ) {
            Ok(_) => (),
            Err(err) => {
                eprintln!("Failed to login to API: {}", err);
                return None;
            }
        }

        match self.api_client.get_last_data() {
            Ok(data) => {
                self.api_data = Some(data.clone());
                return Some(data);
            }
            Err(err) => {
                eprintln!("Failed to get data from API: {}", err);
                return None;
            }
        }
    }
}

#[get("/")]
async fn inverter_current_data(data: Data<Mutex<AppData>>) -> impl Responder {
    let mut app_data = data.lock().unwrap();

    let dtn = Utc::now().naive_utc();

    if let Some(sniffer_data) = &*app_data.sniffer_data.lock().unwrap() {
        if (dtn - sniffer_data.timestamp).num_seconds() < 60 {
            println!("Returning sniffer data, its not older than 60 seconds...");
            return HttpResponse::Ok().json(sniffer_data);
        } else {
            if let Some(api_data) = &app_data.api_data {
                if sniffer_data.timestamp < api_data.timestamp {
                    println!("Returning sniffer data, its not older api data...");
                    return HttpResponse::Ok().json(sniffer_data);
                }
            }
        }
    }

    println!("Don't have sniffer data...");

    if let Some(api_data) = &app_data.api_data {
        if (dtn - api_data.timestamp).num_seconds() < 60 {
            println!("Returning api data, its not older than 60 seconds...");
            let converted = inverter_data_from_watch_power_last_data(api_data);
            return HttpResponse::Ok().json(converted);
        }
    }

    println!("Getting new api data...");
    if let Some(new_api_data) = app_data.update_api_data() {
        let converted = inverter_data_from_watch_power_last_data(&new_api_data);
        return HttpResponse::Ok().json(converted);
    }

    HttpResponse::NotFound().json(json!({"Error": "Resource not found"}))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    println!("Starting sniffer service...");

    let current_inverter_data = start_tcp_sniffer()
        .await
        .expect("Error starting tcp sniffer");

    println!("Sniffer is up!");

    println!("Starting http server...");

    let api = WatchPowerAPI::new(
        &std::env::var("WATCHPOWER_INVERTER_SN").expect("WATCHPOWER_INVERTER_SN must be set."),
        &std::env::var("WATCHPOWER_WIFI_PN").expect("WATCHPOWER_WIFI_PN must be set."),
        std::env::var("WATCHPOWER_DEV_CODE")
            .expect("WATCHPOWER_DEV_CODE must be set.")
            .parse::<i32>()
            .unwrap(),
        std::env::var("WATCHPOWER_DEV_ADDR")
            .expect("WATCHPOWER_DEV_ADDR must be set.")
            .parse::<i32>()
            .unwrap(),
    );
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(Mutex::new(AppData {
                sniffer_data: current_inverter_data.clone(),
                api_data: None,
                api_client: api.clone(),
            })))
            .service(inverter_current_data)
    })
    .bind(("127.0.0.1", 5678))?
    .run()
    .await?;

    println!("Shutting down...");

    Ok(())
}
