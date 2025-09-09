use crate::{
    app_realtime::notifications::notifs_cfg::NotifsCfgResponse_notif_item, global_vars::GlobalVars,
};
use chrono::{NaiveDateTime, NaiveTime, TimeDelta, Utc};
use std::{collections::HashMap, sync::Arc};
use tokio::time::Instant;

#[derive(Debug)]
pub struct NotifCompressorUsedBeforeHour {
    pub notif_id: u64,
    pub last_notif_sent: Option<Instant>,

    pub time_limit: NaiveTime,
}
impl NotifCompressorUsedBeforeHour {
    // "COMP_TIME <" compressorEstiverLigadoAntesDe 'HH:mm'
    pub fn from_notif_cfg(
        updated_notif_data: &Arc<NotifsCfgResponse_notif_item>,
    ) -> Result<Self, String> {
        // Pega o COND_PARS['TIME_LIMIT']
        let time_limit = {
            let time_limit = updated_notif_data.cond_pars["TIME_LIMIT"].as_str();
            let time_limit = match time_limit {
                Some(x) => x,
                None => {
                    return Err("TIME_LIMIT é obrigatório para a notificação".to_owned());
                }
            };
            let time_limit = NaiveTime::parse_from_str(time_limit, "%H:%M")
                .map_err(|err| format!("[134] TIME_LIMIT é inválido: '{time_limit}' {err}"))?;
            time_limit
        };

        Ok(Self {
            notif_id: updated_notif_data.notif_id,
            last_notif_sent: None,
            time_limit,
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

            // Se tiver alterado o horário da notificação, deixa enviar nova notificação hoje ainda
            let changed_time = existing.time_limit != self.time_limit;
            if changed_time {
                self.last_notif_sent = None;
            }
        }
    }

    pub async fn on_dac_telemetry(
        &mut self,
        telemetry_l1: i16,
        telemetry_timestamp: &NaiveDateTime,
        new_day: bool,
        dev_id: &str,
        globs: &Arc<GlobalVars>,
    ) -> Result<(), String> {
        // Se o compressor não estiver ligado não tem nada para conferir
        if telemetry_l1 == 0 {
            return Ok(());
        }

        // Quando troca o dia, libera nova notificação
        if new_day && self.last_notif_sent.is_some() {
            self.last_notif_sent = None;
        }

        // Se já tiver enviado uma notificação menos de 24 horas atrás, não precisa nem conferir
        if let Some(last_notif_sent) = self.last_notif_sent.as_ref() {
            if last_notif_sent.elapsed().as_secs() < 24 * 60 * 60 {
                return Ok(());
            }
        }

        if telemetry_timestamp.time() < self.time_limit {
            self.last_notif_sent = Some(Instant::now());
            let result = globs
                .to_notifs_queue
                .send((
                    "/COMP_TIME/AntesDoHorario",
                    serde_json::json!({
                        "dev_id": dev_id.to_owned(),
                        "notif_id": self.notif_id,
                        "time_limit": self.time_limit.format("%H:%M:%S").to_string(),
                        "telemetry_timestamp": *telemetry_timestamp,
                        "detection_time": Utc::now(),
                    }),
                ))
                .await;
            result.map_err(|err| crate::log_err("[216]", err)).ok();
        }

        return Ok(());
    }
}

#[derive(Debug)]
pub struct NotifCompressorUsedAfterHour {
    pub notif_id: u64,
    pub last_notif_sent: Option<Instant>,

    pub time_limit: NaiveTime,
}
impl NotifCompressorUsedAfterHour {
    // "COMP_TIME >" compressorEstiverLigadoDepoisDe 'HH:mm'
    pub fn from_notif_cfg(
        updated_notif_data: &Arc<NotifsCfgResponse_notif_item>,
    ) -> Result<Self, String> {
        // Pega o COND_PARS['TIME_LIMIT']
        let time_limit = {
            let time_limit = updated_notif_data.cond_pars["TIME_LIMIT"].as_str();
            let time_limit = match time_limit {
                Some(x) => x,
                None => {
                    return Err("TIME_LIMIT é obrigatório para a notificação".to_owned());
                }
            };
            let (time_limit, _) = NaiveTime::parse_from_str(time_limit, "%H:%M")
                .map_err(|err| format!("[134] TIME_LIMIT é inválido: '{time_limit}' {err}"))?
                .overflowing_add_signed(TimeDelta::seconds(59));
            time_limit
        };

        Ok(Self {
            notif_id: updated_notif_data.notif_id,
            last_notif_sent: None,
            time_limit,
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

            // Se tiver alterado o horário da notificação, deixa enviar nova notificação hoje ainda
            let changed_time = existing.time_limit != self.time_limit;
            if changed_time {
                self.last_notif_sent = None;
            }
        }
    }

    pub async fn on_dac_telemetry(
        &mut self,
        telemetry_l1: i16,
        telemetry_timestamp: &NaiveDateTime,
        new_day: bool,
        dev_id: &str,
        globs: &Arc<GlobalVars>,
    ) -> Result<(), String> {
        // Se o compressor não estiver ligado não tem nada para conferir
        if telemetry_l1 == 0 {
            return Ok(());
        }

        // Quando troca o dia, libera nova notificação
        if new_day && self.last_notif_sent.is_some() {
            self.last_notif_sent = None;
        }

        // Se já tiver enviado uma notificação menos de 24 horas atrás, não precisa nem conferir
        if let Some(last_notif_sent) = self.last_notif_sent.as_ref() {
            if last_notif_sent.elapsed().as_secs() < 24 * 60 * 60 {
                return Ok(());
            }
        }

        if telemetry_timestamp.time() > self.time_limit {
            self.last_notif_sent = Some(Instant::now());
            let result = globs
                .to_notifs_queue
                .send((
                    "/COMP_TIME/DepoisDoHorario",
                    serde_json::json!({
                        "dev_id": dev_id.to_owned(),
                        "notif_id": self.notif_id,
                        "time_limit": self.time_limit.format("%H:%M:%S").to_string(),
                        "telemetry_timestamp": *telemetry_timestamp,
                        "detection_time": Utc::now(),
                    }),
                ))
                .await;
            result.map_err(|err| crate::log_err("[216]", err)).ok();
        }

        return Ok(());
    }
}
