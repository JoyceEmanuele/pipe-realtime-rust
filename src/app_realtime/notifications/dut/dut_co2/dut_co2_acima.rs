use crate::{
    app_realtime::notifications::notifs_cfg::{DutAutomationConfig, NotifsCfgResponse_notif_item},
    global_vars::GlobalVars,
};
use chrono::{NaiveDateTime, Utc};
use std::{collections::HashMap, sync::Arc};
use tokio::time::Instant;

#[derive(Debug)]
pub struct NotifDutCO2High {
    pub notif_id: u64,
    pub last_notif_sent: Option<Instant>,
    pub acc_t: u64,
    pub co2max: f64,
}
impl NotifDutCO2High {
    // "DUT_CO2 >"
    pub fn from_notif_cfg(
        updated_notif_data: &Arc<NotifsCfgResponse_notif_item>,
        automation_cfg: &Option<Arc<DutAutomationConfig>>,
    ) -> Result<Self, &'static str> {
        let co2max = automation_cfg.as_ref().and_then(|x| x.co2max);
        let co2max = match co2max {
            Some(x) => x,
            None => {
                return Err("CO2MAX é obrigatório para a notificação");
            }
        };
        Ok(Self {
            notif_id: updated_notif_data.notif_id,
            acc_t: 0,
            last_notif_sent: None,
            co2max,
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
            self.acc_t = existing.acc_t;
        }
    }

    pub async fn on_dut_telemetry(
        &mut self,
        telemetry_co2: f64,
        // telemetry_timestamp: &DateTime<FixedOffset>,
        telemetry_timestamp: &NaiveDateTime,
        delta_secs: u64,
        new_day: bool,
        // descontinuidade: bool,
        dev_id: &str,
        globs: &Arc<GlobalVars>,
    ) -> Result<(), String> {
        // { label: 'DUT_CO2.nivelCO2.estiverAcimaLimite', value: '>', unit: null, describe: (val) => !req? `Nível de CO2 estiver acima do limite` : 'notificacao.nivelCO2AcimaLimite' },

        if new_day {
            self.acc_t = 0;
        }

        // Se já tiver enviado uma notificação menos de 24 horas atrás, não precisa nem conferir
        if let Some(last_notif_sent) = self.last_notif_sent.as_ref() {
            if last_notif_sent.elapsed().as_secs() < 24 * 60 * 60 {
                // if (row.lastNotifSent > limit24h) continue
                return Ok(());
            }
        }

        if telemetry_co2 > self.co2max {
            if self.acc_t > (10 * 60) {
                self.last_notif_sent = Some(Instant::now());
                let result = globs
                    .to_notifs_queue
                    .send((
                        "/DUT_CO2/Acima",
                        serde_json::json!({
                            "dev_id": dev_id.to_owned(),
                            "notif_id": self.notif_id,
                            "CO2MAX": self.co2max,
                            "telemetry_timestamp": *telemetry_timestamp,
                            "detection_time": Utc::now(),
                        }),
                    ))
                    .await;
                result.map_err(|err| crate::log_err("[439]", err)).ok();
            } else {
                self.acc_t += delta_secs;
            }
        }

        return Ok(());
    }
}
