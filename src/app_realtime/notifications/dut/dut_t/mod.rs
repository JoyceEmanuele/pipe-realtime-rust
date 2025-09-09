use super::super::get_telemetry_delta;
use super::notifs_dut::NotifsDut;
use super::on_dut_telemetry::DutTelemetry;
use crate::global_vars::GlobalVars;
use chrono::NaiveDateTime;
use std::sync::Arc;

pub use dut_t_acima_limite_critico::NotifDutTempHighCritic;
pub use dut_t_fora_limites_antigo::NotifDutTempOutOfBounds;

pub mod dut_t_acima_limite_critico;
pub mod dut_t_fora_limites_antigo;

#[derive(Debug)]
pub struct LastDutTemperature {
    pub timestamp: NaiveDateTime,
    pub temperature: f64,
}

pub async fn on_dut_telemetry(
    telemetry: &DutTelemetry,
    dev_alerts: &mut NotifsDut,
    dev_id: &str,
    // telemetry_timestamp: &DateTime<FixedOffset>,
    globs: &Arc<GlobalVars>,
) {
    // Todas as verificações aqui são baseadas na temperatura, se não tiver pode interromper
    let Some(telemetry_temperature) = telemetry.Temperature else {
        return;
    };
    let telemetry_timestamp = &telemetry.timestamp;

    let prev_value_timestamp = dev_alerts.last_temperature.as_ref().map(|x| &x.timestamp);
    let (delta_secs, descontinuidade, new_day) =
        get_telemetry_delta(telemetry_timestamp, prev_value_timestamp, 30);

    // Atualiza o dev_alerts.last_temperature
    let mut prev_temperature = None;
    if let Some(last_temperature) = dev_alerts.last_temperature.as_mut() {
        prev_temperature = Some(last_temperature.temperature);
        last_temperature.temperature = telemetry_temperature;
        last_temperature.timestamp = telemetry_timestamp.to_owned();
    } else {
        dev_alerts.last_temperature = Some(LastDutTemperature {
            temperature: telemetry_temperature,
            timestamp: telemetry_timestamp.to_owned(),
        });
    }

    if descontinuidade {
        prev_temperature = None;
    }

    for (_, row) in dev_alerts.notif_dut_temp_outofbounds.iter_mut() {
        row.on_dut_telemetry(
            telemetry_temperature,
            telemetry_timestamp,
            delta_secs,
            new_day,
            descontinuidade,
            dev_id,
            globs,
        )
        .await
        .map_err(|err| crate::log_err("[130]", err))
        .ok();
    }

    for (_, row) in dev_alerts.notif_dut_temp_high_critic.iter_mut() {
        row.on_dut_telemetry(
            telemetry_temperature,
            telemetry_timestamp,
            delta_secs,
            prev_temperature,
            dev_id,
            globs,
        )
        .await
        .map_err(|err| crate::log_err("[142]", err))
        .ok();
    }
}
