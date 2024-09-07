use chrono::{NaiveDateTime, Utc};
use mysql::{params, prelude::Queryable, Conn};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct InverterStatusQPIGS {
    pub add_sbu_priority_version: bool,
    pub config_changed: bool,
    pub scc_firmware_updates: bool,
    pub load_on: bool,
    pub bat_volt_to_steady: bool,
    pub charging: bool,
    pub charging_scc: bool,
    pub charging_ac: bool,
    pub charging_to_floating_point: bool,
    pub switch_on: bool,
    pub reserved: bool,
}

impl InverterStatusQPIGS {
    pub fn from_bitfield(field: &str) -> Result<Self, String> {
        let mut chars = field.chars();
        Ok(Self {
            add_sbu_priority_version: chars
                .next()
                .ok_or("Error parsing add_sbu_priority_version, no more chars!")?
                == '1',
            config_changed: chars
                .next()
                .ok_or("Error parsing config_changed, no more chars!")?
                == '1',
            scc_firmware_updates: chars
                .next()
                .ok_or("Error parsing scc_firmware_updates, no more chars!")?
                == '1',
            load_on: chars
                .next()
                .ok_or("Error parsing load_on, no more chars!")?
                == '1',
            bat_volt_to_steady: chars
                .next()
                .ok_or("Error parsing bat_volt_to_steady, no more chars!")?
                == '1',
            charging: chars
                .next()
                .ok_or("Error parsing charging, no more chars!")?
                == '1',
            charging_scc: chars
                .next()
                .ok_or("Error parsing charging_scc, no more chars!")?
                == '1',
            charging_ac: chars
                .next()
                .ok_or("Error parsing charging_ac, no more chars!")?
                == '1',
            charging_to_floating_point: chars
                .next()
                .ok_or("Error parsing charging_to_floating_point, no more chars!")?
                == '1',
            switch_on: chars
                .next()
                .ok_or("Error parsing switch_on, no more chars!")?
                == '1',
            reserved: chars
                .next()
                .ok_or("Error parsing reserved, no more chars!")?
                == '1',
        })
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct InverterDataQPIGS {
    pub timestamp: NaiveDateTime,
    pub grid_voltage: f32,
    pub grid_frequency: f32,
    pub ac_output_voltage: f32,
    pub ac_output_frequency: f32,
    pub ac_output_apparent_power: u16,
    pub ac_output_active_power: f32,
    pub ac_output_load_percent: f32,
    pub bus_voltage: f32,
    pub bat_voltage: f32,
    pub bat_charge_current: f32,
    pub bat_capacity: f32,
    pub heat_sink_temp: f32,
    pub pv_current: f32,
    pub pv_voltage: f32,
    pub bat_voltage_from_scc: f32,
    pub bat_discharge_current: f32,
    pub bat_volt_offset: f32,
    pub eeprom_version: u16,
    pub pv_power: u16,
    pub status: InverterStatusQPIGS,
}

impl InverterDataQPIGS {
    // "000.0 00.0 230.1 50.0 0230 0155 004 338 49.20 000 053 0029 00.0 000.0 00.00 00004 10010000 00 00 00000 011"
    pub fn from_packet(bytes: &[u8]) -> Result<Self, String> {
        let packet = String::from_utf8_lossy(bytes);
        let index = packet.find("(").ok_or("Could not find start byte")?;
        let actual_information = packet.get(index + 1..index + 1 + 106).ok_or(format!(
            "String is too short to get the last {} to {} characters",
            index + 1,
            index + 1 + 106
        ))?;
        let mut iter = actual_information.split_ascii_whitespace();

        macro_rules! parse_field {
            ($field:ident, $type:ty) => {
                let $field = iter
                    .next()
                    .ok_or(format!("Exhausted tokens on {}", stringify!($field)))?
                    .parse::<$type>()
                    .map_err(|err| format!("Could not parse {}: {}", stringify!($field), err))?;
            };
        }

        parse_field!(grid_voltage, f32);
        parse_field!(grid_frequency, f32);
        parse_field!(ac_output_voltage, f32);
        parse_field!(ac_output_frequency, f32);
        parse_field!(ac_output_apparent_power, u16);
        parse_field!(ac_output_active_power, f32);
        parse_field!(ac_output_load_percent, f32);
        parse_field!(bus_voltage, f32);
        parse_field!(bat_voltage, f32);
        parse_field!(bat_charge_current, f32);
        parse_field!(bat_capacity, f32);
        parse_field!(heat_sink_temp, f32);
        parse_field!(pv_current, f32);
        parse_field!(pv_voltage, f32);
        parse_field!(bat_voltage_from_scc, f32);
        parse_field!(bat_discharge_current, f32);
        let device_status_1: &str = iter.next().ok_or("Exhausted tokens on device_status_1")?;
        parse_field!(bat_volt_offset, f32);
        parse_field!(eeprom_version, u16);
        parse_field!(pv_power, u16);
        let device_status_2: &str = iter.next().ok_or("Exhausted tokens on device_status_2")?;

        let device_status: String = format!("{}{}", device_status_1, device_status_2);

        let status: InverterStatusQPIGS = InverterStatusQPIGS::from_bitfield(&device_status)
            .map_err(|err| format!("Error getting inverter status: {err}"))?;

        Ok(Self {
            timestamp: Utc::now().naive_utc(),
            grid_voltage,
            grid_frequency,
            ac_output_voltage,
            ac_output_frequency,
            ac_output_apparent_power,
            ac_output_active_power,
            ac_output_load_percent,
            bus_voltage,
            bat_voltage,
            bat_charge_current,
            bat_capacity,
            heat_sink_temp,
            pv_current,
            pv_voltage,
            bat_voltage_from_scc,
            bat_discharge_current,
            bat_volt_offset,
            eeprom_version,
            pv_power,
            status,
        })
    }

    pub fn to_mysql(self, conn: &mut Conn) {
        let stmt: &str = r"insert into stats
            (inverter_id, grid_voltage, grid_frequency, ac_output_voltage, ac_output_frequency,
                ac_output_apparent_power, ac_output_active_power, ac_output_load_percent, bus_voltage, bat_voltage,
                bat_current, bat_capacity, heat_sink_temp, pv_current, pv_voltage, pv_power, bat_voltage_from_scc,
                load_on, bat_voltage_to_steady, charging_on, charge_scc_on, charge_ac_on, charging_to_floating_point,
                switch_on)
                values (1, :grid_voltage, :grid_frequency, :ac_output_voltage, :ac_output_frequency,
                    :ac_output_apparent_power, :ac_output_active_power, :ac_output_load_percent, :bus_voltage, :bat_voltage, :bat_current, :bat_capacity, :heat_sink_temp, :pv_current, :pv_voltage, :pv_power, :bat_voltage_from_scc, :load_on, :bat_voltage_to_steady, :charging_on, :charge_scc_on, :charge_ac_on, :charging_to_floating_point, :switch_on);";

        let _ = conn.exec_drop(
            stmt,
            params! {
                "grid_voltage" => self.grid_voltage,
                "grid_frequency" => self.grid_frequency,
                "ac_output_voltage" => self.ac_output_voltage,
                "ac_output_frequency" => self.ac_output_frequency,
                "ac_output_apparent_power" => self.ac_output_apparent_power,
                "ac_output_active_power" => self.ac_output_active_power,
                "ac_output_load_percent" => self.ac_output_load_percent,
                "bus_voltage" => self.bus_voltage,
                "bat_voltage" => self.bat_voltage,
                "bat_current" => self.bat_charge_current - self.bat_discharge_current,
                "bat_capacity" => self.bat_capacity,
                "heat_sink_temp" => self.heat_sink_temp,
                "pv_current" => self.pv_current,
                "pv_voltage" => self.pv_voltage,
                "pv_power" => self.pv_power,
                "bat_voltage_from_scc" => self.bat_voltage_from_scc,
                "load_on" => self.status.load_on,
                "bat_voltage_to_steady" => self.status.bat_volt_to_steady,
                "charging_on" => self.status.charging,
                "charge_scc_on" => self.status.charging_scc,
                "charge_ac_on" => self.status.charging_ac,
                "charging_to_floating_point" => self.status.charging_to_floating_point,
                "switch_on" => self.status.switch_on
            },
        );
    }
}
