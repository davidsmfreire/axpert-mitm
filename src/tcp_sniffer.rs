use std::{
    sync::{Arc, Mutex},
    thread,
};

use mysql::Pool;
use pcap::{Capture, Device};

use crate::models::InverterDataQPIGS;

fn run_tcp_sniffer(
    mut cap: Capture<pcap::Active>,
    pool: Pool,
    current_inverter_data: Arc<Mutex<Option<InverterDataQPIGS>>>,
) -> std::io::Result<()> {
    while let Ok(packet) = cap.next_packet() {
        // println!("Captured packet: {:?}", packet.header);
        if packet.header.len == 172 {
            match InverterDataQPIGS::from_packet(&packet.data) {
                Ok(inverter_data) => {
                    println!("Inverter data update: {inverter_data:?}");

                    let mut curr_inverter_data = current_inverter_data.lock().unwrap();
                    *curr_inverter_data = Some(inverter_data.clone());

                    let conn = pool.get_conn();
                    if let Ok(mut conn) = conn {
                        inverter_data.to_mysql(conn.as_mut());
                    }
                }
                Err(err) => {
                    eprintln!("Error parsing inverter data: {}", err);
                }
            }
        }
    }
    Ok(())
}

pub(crate) async fn start_tcp_sniffer() -> Result<Arc<Mutex<Option<InverterDataQPIGS>>>, String> {
    let db_url: String = std::env::var("DB_URL").expect("DB_URL must be set.");
    let mut lst = Device::list().unwrap();
    let mut target_device: Option<Device> = None;
    for device in lst.iter_mut() {
        if device.name == "wlan0" {
            target_device = Some(device.clone());
        }
    }
    let actual_device = target_device.ok_or("Target device wlan0 not found")?;

    println!("Starting wlan0 capture...");

    let mut cap = Capture::from_device(actual_device)
        .unwrap()
        .promisc(true) // Enable promiscuous mode
        .immediate_mode(true)
        .open()
        .map_err(|err| format!("Error initiating capture: {err}"))?;

    // Set a filter to capture only TCP packets
    cap.filter("tcp and dst host 47.242.188.205 and dst port 502", false)
        .unwrap();

    println!("Connecting to MySQL...");
    let db_connection_pool: Pool =
        Pool::new(db_url.as_str()).expect("Could not connect to database");

    let current_inverter_data = Arc::new(Mutex::new(None));
    let current_inverter_data_clone = current_inverter_data.clone();

    println!("Starting tcp sniffer thread...");

    thread::spawn(|| run_tcp_sniffer(cap, db_connection_pool, current_inverter_data_clone));

    Ok(current_inverter_data.clone())
}
