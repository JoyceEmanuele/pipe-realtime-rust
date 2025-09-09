use super::dut_co2;
use super::dut_t;
use super::notifs_dut::NotifsDut;
use crate::global_vars::GlobalVars;
use crate::helpers::telemetry_payloads::{
    parse_json_props::{get_float_array_optional, get_i16_array_optional},
    telemetry_formats::{get_json_sampling_time, get_json_timestamp_with_gmt},
};
use chrono::{NaiveDateTime, TimeDelta};
use std::sync::Arc;

pub async fn on_dut_telemetry(
    payload_json: &serde_json::Value,
    dev_alerts: &mut NotifsDut,
    dev_id: &str,
    globs: &Arc<GlobalVars>,
) {
    // Se a telemetria for histórica acho que posso ignorar.

    let telemetry_list = match DutTelemetry::parse_from_json_as_vec(payload_json) {
        Err(err) => {
            crate::write_to_log_file_v2(
                "ERROR",
                &format!("[102] {err} {}", payload_json.to_string()),
                false,
            );
            return;
        }
        Ok(x) => x,
    };

    for telemetry in telemetry_list {
        // O DUT tem que ter um horário de funcionamento definido para o dia do timestamp da telemetria
        let Some(schedule) = dev_alerts.schedule.as_ref() else {
            continue;
        };
        let Some(current) = schedule.get_for(&telemetry.timestamp.date()) else {
            continue;
        };

        // Todas as verificações abaixo só são executadas dentro do horário de funcionamento
        let inside_schedule = current.is_inside_sched(telemetry.timestamp.time());
        if !inside_schedule {
            continue;
        }

        // Notificações sobre a temperatura
        dut_t::on_dut_telemetry(&telemetry, dev_alerts, dev_id, globs).await;

        // Notificações sobre o CO2
        dut_co2::on_dut_telemetry(&telemetry, dev_alerts, dev_id, globs).await;
    }
}

pub struct DutTelemetry {
    pub timestamp: NaiveDateTime,
    pub Temperature: Option<f64>,
    pub eCO2: Option<i16>,
}

impl DutTelemetry {
    pub fn parse_from_json_as_vec(payload_json: &serde_json::Value) -> Result<Vec<Self>, String> {
        let (payload_timestamp, gmt) = get_json_timestamp_with_gmt(payload_json)?;
        let sampling_time = get_json_sampling_time(&payload_json).unwrap_or(5);

        let Temperature = get_float_array_optional(&payload_json["Temperature"]);
        let eCO2 = get_i16_array_optional(&payload_json["eCO2"]);

        let vec_len = Temperature
            .as_ref()
            .and_then(|v| Some(v.len()))
            .or_else(|| eCO2.as_ref().and_then(|v| Some(v.len())))
            .unwrap_or(0);

        let mut vec = Vec::with_capacity(vec_len);

        for i in 0..vec_len {
            let sub_times = (vec_len - 1 - i) as i64;
            let telemetry_timestamp = payload_timestamp
                .checked_sub_signed(TimeDelta::seconds(sub_times * sampling_time))
                .unwrap();

            let telemetry = DutTelemetry {
                Temperature: Temperature.as_ref().and_then(|x| x.get(i).and_then(|x| *x)),
                eCO2: eCO2.as_ref().and_then(|x| x.get(i).and_then(|x| *x)),
                timestamp: telemetry_timestamp,
            };
            vec.push(telemetry);
        }
        Ok(vec)
    }
}
