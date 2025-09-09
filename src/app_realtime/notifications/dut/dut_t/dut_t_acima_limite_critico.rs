use crate::{
    app_realtime::notifications::notifs_cfg::{DutAutomationConfig, NotifsCfgResponse_notif_item},
    global_vars::GlobalVars,
};
use chrono::{NaiveDateTime, Utc};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct NotifDutTempHighCritic {
    pub notif_id: u64,
    pub acc_t: u64,
    pub is_cond_notification: bool,
    pub temperature_limit: f64,
    pub duration_in_seconds: u64,
}

impl NotifDutTempHighCritic {
    // "DUT_T T>T"
    pub fn from_notif_cfg(
        updated_notif_data: &Arc<NotifsCfgResponse_notif_item>,
        automation_cfg: &Option<Arc<DutAutomationConfig>>,
    ) -> Result<Self, &'static str> {
        let tusemax = automation_cfg.as_ref().and_then(|x| x.tusemax);
        let tusemax = match tusemax {
            Some(x) => x,
            None => {
                return Err("TUSEMAX é obrigatório para a notificação");
            }
        };
        // OFFSET_OVER_T_MAX é o offset de temperatura além do limite máximo do ambiente
        let mut offset_over_t_max = updated_notif_data.cond_pars["OFFSET_OVER_T_MAX"]
            .as_f64()
            .unwrap_or(0.0);
        // DURATION_IN_MINUTES é o tempo em minutos para disparar o alerta
        let duration_in_minutes = updated_notif_data.cond_pars["DURATION_IN_MINUTES"]
            .as_u64()
            .unwrap_or(10);

        if offset_over_t_max < 0.0 {
            offset_over_t_max = 0.0;
        }

        let temperature_limit = tusemax + offset_over_t_max;
        let duration_in_seconds = duration_in_minutes * 60;

        Ok(Self {
            notif_id: updated_notif_data.notif_id,
            acc_t: 0,
            is_cond_notification: false,
            temperature_limit,
            duration_in_seconds,
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
            self.acc_t = existing.acc_t;
            self.is_cond_notification = existing.is_cond_notification;
        }
    }

    pub async fn on_dut_telemetry(
        &mut self,
        telemetry_temperature: f64,
        // telemetry_timestamp: &DateTime<FixedOffset>,
        telemetry_timestamp: &NaiveDateTime,
        delta_secs: u64,
        prev_temperature: Option<f64>,
        dev_id: &str,
        globs: &Arc<GlobalVars>,
    ) -> Result<(), String> {
        // { label: 'DUT_T.temperaturaAmbiente.estiverAcimaLimiteCritica', value: 'T>T', unit: 'notificacao.offsetLimiteSuperiorTemperatura', unit2: 'notificacao.duraçãoMinimaAcumuladaMin', describe: (val) => !req? `Temperatura de Ambiente acima do limite estabelecido` : 'notificacao.temperaturaAmbienteAcimaDosLimites' },

        // if new_day || descontinuidade {
        //     self.acc_t = 0;
        // }

        // Se já tiver enviado uma notificação menos de 24 horas atrás, não precisa nem conferir
        // if let Some(last_notif_sent) = self.last_notif_sent.as_ref() {
        //     if last_notif_sent.elapsed().as_secs() < 24 * 60 * 60 {
        //         continue;
        //     }
        // }

        let curr_temp_above = telemetry_temperature > self.temperature_limit;
        let prev_temp_above = prev_temperature
            .map(|t| t > self.temperature_limit)
            .unwrap_or(false);

        if (curr_temp_above && !prev_temp_above) || self.is_cond_notification {
            if telemetry_temperature <= self.temperature_limit {
                self.is_cond_notification = false;
                self.acc_t = 0;
            } else {
                self.is_cond_notification = true;
                if self.acc_t > self.duration_in_seconds {
                    self.is_cond_notification = false;
                    self.acc_t = 0;
                    // self.last_notif_sent = Some(Instant::now());
                    let result = globs
                        .to_notifs_queue
                        .send((
                            "/DUT_T/AcimaLimiteCritica",
                            serde_json::json!({
                                "dev_id": dev_id.to_owned(),
                                "notif_id": self.notif_id,
                                "telemetry_timestamp": *telemetry_timestamp,
                                "temperature_limit": self.temperature_limit,
                                "duration_in_seconds": self.duration_in_seconds,
                                "detection_time": Utc::now(),
                            }),
                        ))
                        .await;
                    result.map_err(|err| crate::log_err("[360]", err)).ok();
                } else {
                    self.acc_t += delta_secs;
                }
            }
        }

        return Ok(());
    }
}
