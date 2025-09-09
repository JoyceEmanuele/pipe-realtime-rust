use super::super::get_telemetry_delta;
use super::notifs_dut::NotifsDut;
use super::on_dut_telemetry::DutTelemetry;
use crate::global_vars::GlobalVars;
use chrono::NaiveDateTime;
use std::sync::Arc;

pub use dut_co2_acima::NotifDutCO2High;
pub use dut_co2_acima_diario::NotifDutCO2HighEndOfDay;

pub mod dut_co2_acima;
pub mod dut_co2_acima_diario;

#[derive(Debug)]
pub struct LastDutCO2 {
    pub timestamp: NaiveDateTime,
    pub co2: f64,
}

pub async fn on_dut_telemetry(
    telemetry: &DutTelemetry,
    dev_alerts: &mut NotifsDut,
    dev_id: &str,
    // telemetry_timestamp: &DateTime<FixedOffset>,
    globs: &Arc<GlobalVars>,
) {
    let Some(telemetry_co2) = telemetry.eCO2 else {
        // Todas as verificações aqui são baseadas no CO2, se não tiver CO2 pode interromper
        return;
    };
    let telemetry_co2 = telemetry_co2 as f64;
    let telemetry_timestamp = &telemetry.timestamp;

    let prev_value_timestamp = dev_alerts.last_co2.as_ref().map(|x| &x.timestamp);
    let (delta_secs, _descontinuidade, new_day) =
        get_telemetry_delta(telemetry_timestamp, prev_value_timestamp, 3 * 60);

    if let Some(last_co2) = dev_alerts.last_co2.as_mut() {
        last_co2.co2 = telemetry_co2;
        last_co2.timestamp = telemetry_timestamp.to_owned();
    } else {
        dev_alerts.last_co2 = Some(LastDutCO2 {
            co2: telemetry_co2,
            timestamp: telemetry_timestamp.to_owned(),
        });
    }

    for (_, row) in dev_alerts.notif_dut_co2_high.iter_mut() {
        row.on_dut_telemetry(
            telemetry_co2,
            telemetry_timestamp,
            delta_secs,
            new_day,
            // descontinuidade,
            &dev_id,
            globs,
        )
        .await
        .map_err(|err| crate::log_err("[157]", err))
        .ok();
    }

    for (_, row) in dev_alerts.notif_dut_co2_high_endofday.iter_mut() {
        row.on_dut_telemetry(
            telemetry_co2,
            telemetry_timestamp,
            delta_secs,
            new_day,
            // descontinuidade,
            &dev_id,
            globs,
        )
        .await
        .map_err(|err| crate::log_err("[172]", err))
        .ok();
    }
}
