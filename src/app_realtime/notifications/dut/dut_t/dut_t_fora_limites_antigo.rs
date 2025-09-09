use crate::{
    app_realtime::notifications::notifs_cfg::{DutAutomationConfig, NotifsCfgResponse_notif_item},
    global_vars::GlobalVars,
};
use chrono::{NaiveDateTime, Utc};
use std::{collections::HashMap, sync::Arc};
use tokio::time::Instant;

#[derive(Debug)]
pub struct NotifDutTempOutOfBounds {
    pub notif_id: u64,
    pub last_notif_sent: Option<Instant>,
    pub seconds_above: u64,
    pub seconds_below: u64,
    pub tusemax: Option<f64>,
    pub tusemin: Option<f64>,
}
impl NotifDutTempOutOfBounds {
    // "DUT_T T<>T"
    pub fn from_notif_cfg(
        updated_notif_data: &Arc<NotifsCfgResponse_notif_item>,
        automation_cfg: &Option<Arc<DutAutomationConfig>>,
    ) -> Result<Self, &'static str> {
        let tusemax = automation_cfg.as_ref().and_then(|x| x.tusemax);
        let tusemin = automation_cfg.as_ref().and_then(|x| x.tusemin);
        if tusemax.is_none() && tusemin.is_none() {
            return Err("Nenhum limite definido");
        }
        Ok(Self {
            notif_id: updated_notif_data.notif_id,
            seconds_above: 0,
            seconds_below: 0,
            last_notif_sent: None,
            tusemax: tusemax.to_owned(),
            tusemin: tusemin.to_owned(),
        })
    }

    pub fn update_new_notifs_parameters(
        list_old: &HashMap<u64, Self>,
        list_new: &mut HashMap<u64, Self>,
    ) {
        for (notif_id, updated_notif) in list_new.iter_mut() {
            updated_notif.update_notif_parameters(list_old.get(&notif_id));
        }
    }

    pub fn update_notif_parameters(&mut self, existing: Option<&Self>) {
        if let Some(existing) = existing {
            self.last_notif_sent = existing.last_notif_sent;
            self.seconds_above = existing.seconds_above;
            self.seconds_below = existing.seconds_below;
        }
    }

    pub async fn on_dut_telemetry(
        &mut self,
        // dev_alerts: &mut NotifsDut,
        telemetry_temperature: f64,
        telemetry_timestamp: &NaiveDateTime,
        // telemetry_timestamp: &DateTime<FixedOffset>,
        delta_secs: u64,
        new_day: bool,
        descontinuidade: bool,
        dev_id: &str,
        globs: &Arc<GlobalVars>,
    ) -> Result<(), String> {
        if new_day || descontinuidade {
            self.seconds_above = 0;
            self.seconds_below = 0;
        }

        // Se já tiver enviado uma notificação menos de 24 horas atrás, não precisa nem conferir
        if let Some(last_notif_sent) = self.last_notif_sent.as_ref() {
            if last_notif_sent.elapsed().as_secs() < 24 * 60 * 60 {
                return Ok(());
            }
        }

        if let Some(tusemax) = self.tusemax {
            if telemetry_temperature > tusemax {
                if self.seconds_above > (10 * 60) {
                    self.last_notif_sent = Some(Instant::now());
                    let result = globs
                        .to_notifs_queue
                        .send((
                            "/DUT_T/AcimaLimiteAntiga",
                            serde_json::json!({
                                "dev_id": dev_id.to_owned(),
                                "notif_id": self.notif_id,
                                "TUSEMAX": tusemax,
                                "telemetry_timestamp": *telemetry_timestamp,
                                "detection_time": Utc::now(),
                            }),
                        ))
                        .await;
                    result.map_err(|err| crate::log_err("[216]", err)).ok();
                } else {
                    self.seconds_above += delta_secs;
                }
            }
        };

        if let Some(tusemin) = self.tusemin {
            if telemetry_temperature < tusemin {
                if self.seconds_below > (10 * 60) {
                    self.last_notif_sent = Some(Instant::now());
                    let result = globs
                        .to_notifs_queue
                        .send((
                            "/DUT_T/AbaixoLimiteAntiga",
                            serde_json::json!({
                                "dev_id": dev_id.to_owned(),
                                "notif_id": self.notif_id,
                                "TUSEMIN": tusemin,
                                "telemetry_timestamp": *telemetry_timestamp,
                                "detection_time": Utc::now(),
                            }),
                        ))
                        .await;
                    result.map_err(|err| crate::log_err("[244]", err)).ok();
                } else {
                    self.seconds_below += delta_secs;
                }
            }
        };

        return Ok(());
    }
}
