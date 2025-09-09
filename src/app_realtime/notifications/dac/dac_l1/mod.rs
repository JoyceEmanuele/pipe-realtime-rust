use super::super::get_telemetry_delta;
use super::notifs_dac::NotifsDac;
use super::on_dac_telemetry::DacTelemetry;
use crate::global_vars::GlobalVars;
use chrono::NaiveDateTime;
use std::sync::Arc;

pub mod dac_compressor_usage_hours;

#[derive(Debug)]
pub struct LastDacL1 {
    pub timestamp: NaiveDateTime,
}

pub async fn on_dac_telemetry(
    telemetry: &DacTelemetry,
    dev_alerts: &mut NotifsDac,
    dev_id: &str,
    globs: &Arc<GlobalVars>,
) {
    // Todas as verificações aqui são baseadas no L1, se não tiver pode interromper
    let Some(telemetry_l1) = telemetry.Lcmp else {
        return;
    };
    let telemetry_timestamp = &telemetry.timestamp;

    let prev_value_timestamp = dev_alerts.last_l1.as_ref().map(|x| &x.timestamp);
    let (delta_secs, descontinuidade, new_day) =
        get_telemetry_delta(telemetry_timestamp, prev_value_timestamp, 30);

    // Atualiza o dev_alerts.last_l1
    if let Some(last_l1) = dev_alerts.last_l1.as_mut() {
        last_l1.timestamp = telemetry_timestamp.to_owned();
    } else {
        dev_alerts.last_l1 = Some(LastDacL1 {
            timestamp: telemetry_timestamp.to_owned(),
        });
    }

    for (_, row) in dev_alerts.notif_compressor_used_before_time.iter_mut() {
        row.on_dac_telemetry(telemetry_l1, telemetry_timestamp, new_day, dev_id, globs)
            .await
            .map_err(|err| crate::log_err("[130]", err))
            .ok();
    }

    for (_, row) in dev_alerts.notif_compressor_used_after_time.iter_mut() {
        row.on_dac_telemetry(telemetry_l1, telemetry_timestamp, new_day, dev_id, globs)
            .await
            .map_err(|err| crate::log_err("[130]", err))
            .ok();
    }
}
