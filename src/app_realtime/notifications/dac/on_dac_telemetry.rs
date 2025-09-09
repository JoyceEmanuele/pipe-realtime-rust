use super::dac_l1;
use super::notifs_dac::NotifsDac;
use crate::global_vars::GlobalVars;
use crate::helpers::telemetry_payloads::{
    parse_json_props::get_i16_array_optional,
    telemetry_formats::{get_json_sampling_time, get_json_timestamp_with_gmt},
};
use chrono::{NaiveDateTime, TimeDelta};
use std::sync::Arc;

pub async fn on_dac_telemetry(
    payload_json: &serde_json::Value,
    dev_alerts: &mut NotifsDac,
    dev_id: &str,
    globs: &Arc<GlobalVars>,
) {
    // Se a telemetria for histórica acho que posso ignorar.

    let telemetry_list = match DacTelemetry::parse_from_json_as_vec(payload_json) {
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
        // Notificações sobre o L1
        dac_l1::on_dac_telemetry(&telemetry, dev_alerts, dev_id, globs).await;
    }
}

pub struct DacTelemetry {
    pub timestamp: NaiveDateTime,
    pub Lcmp: Option<i16>,
}

impl DacTelemetry {
    pub fn parse_from_json_as_vec(payload_json: &serde_json::Value) -> Result<Vec<Self>, String> {
        let (payload_timestamp, gmt) = get_json_timestamp_with_gmt(payload_json)?;
        let sampling_time = get_json_sampling_time(&payload_json).unwrap_or(1);

        let Lcmp = get_i16_array_optional(&payload_json["Lcmp"]);

        let vec_len = Lcmp.as_ref().and_then(|v| Some(v.len())).unwrap_or(0);

        let mut vec = Vec::with_capacity(vec_len);

        for i in 0..vec_len {
            let sub_times = (vec_len - 1 - i) as i64;
            let telemetry_timestamp = payload_timestamp
                .checked_sub_signed(TimeDelta::seconds(sub_times * sampling_time))
                .unwrap();

            let telemetry = DacTelemetry {
                Lcmp: Lcmp.as_ref().and_then(|x| x.get(i).and_then(|x| *x)),
                timestamp: telemetry_timestamp,
            };
            vec.push(telemetry);
        }
        Ok(vec)
    }
}
