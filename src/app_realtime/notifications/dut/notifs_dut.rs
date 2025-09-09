use super::dut_co2::LastDutCO2;
use super::dut_co2::{NotifDutCO2High, NotifDutCO2HighEndOfDay};
use super::dut_t::LastDutTemperature;
use super::dut_t::{NotifDutTempHighCritic, NotifDutTempOutOfBounds};
use crate::app_realtime::global_vars::DevInfo;
use crate::app_realtime::notifications::notifs_cfg::{
    AutomationSchedule, DutAutomationConfig, NotifsCfgResponse_notif_item,
};
use std::sync::atomic::Ordering;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct NotifsDut {
    pub notif_dut_temp_outofbounds: HashMap<u64, NotifDutTempOutOfBounds>,
    pub notif_dut_temp_high_critic: HashMap<u64, NotifDutTempHighCritic>,
    pub notif_dut_co2_high: HashMap<u64, NotifDutCO2High>,
    pub notif_dut_co2_high_endofday: HashMap<u64, NotifDutCO2HighEndOfDay>,
    pub last_temperature: Option<LastDutTemperature>,
    pub last_co2: Option<LastDutCO2>,
    pub schedule: Option<Arc<AutomationSchedule>>,
}
impl NotifsDut {
    pub fn remove_notif_id(&mut self, notif_id: u64) {
        self.notif_dut_temp_outofbounds.remove(&notif_id);
        self.notif_dut_temp_high_critic.remove(&notif_id);
        self.notif_dut_co2_high.remove(&notif_id);
        self.notif_dut_co2_high_endofday.remove(&notif_id);
    }
}

fn parse_dut_notifs_list(
    updated_dev_notifs: &Vec<Arc<NotifsCfgResponse_notif_item>>,
    updated_dev_sched: &Option<Arc<DutAutomationConfig>>,
) -> Option<(
    HashMap<u64, NotifDutTempOutOfBounds>,
    HashMap<u64, NotifDutTempHighCritic>,
    HashMap<u64, NotifDutCO2High>,
    HashMap<u64, NotifDutCO2HighEndOfDay>,
)> {
    let mut notif_dut_temp_outofbounds = HashMap::new();
    let mut notif_dut_temp_high_critic = HashMap::new();
    let mut notif_dut_co2_high = HashMap::new();
    let mut notif_dut_co2_high_endofday = HashMap::new();

    for notif in updated_dev_notifs.iter() {
        match notif.cond_id.as_str() {
            "DUT_T T<>T" => {
                match NotifDutTempOutOfBounds::from_notif_cfg(notif, &updated_dev_sched) {
                    Ok(notif) => {
                        notif_dut_temp_outofbounds.insert(notif.notif_id, notif);
                    }
                    Err(_err) => {} // Ignora silenciosamente
                };
            }
            "DUT_T T>T" => {
                match NotifDutTempHighCritic::from_notif_cfg(notif, &updated_dev_sched) {
                    Ok(notif) => {
                        notif_dut_temp_high_critic.insert(notif.notif_id, notif);
                    }
                    Err(_err) => {} // Ignora silenciosamente
                };
            }
            "DUT_CO2 >" => {
                match NotifDutCO2High::from_notif_cfg(notif, &updated_dev_sched) {
                    Ok(notif) => {
                        notif_dut_co2_high.insert(notif.notif_id, notif);
                    }
                    Err(_err) => {} // Ignora silenciosamente
                };
            }
            "DUT_CO2 D>" => {
                match NotifDutCO2HighEndOfDay::from_notif_cfg(notif, &updated_dev_sched) {
                    Ok(notif) => {
                        notif_dut_co2_high_endofday.insert(notif.notif_id, notif);
                    }
                    Err(_err) => {} // Ignora silenciosamente
                };
            }
            _ => {
                // Ignore
            }
        }
    }

    let is_empty = notif_dut_temp_outofbounds.is_empty()
        && notif_dut_temp_high_critic.is_empty()
        && notif_dut_co2_high.is_empty()
        && notif_dut_co2_high_endofday.is_empty();

    if is_empty {
        return None;
    }

    Some((
        notif_dut_temp_outofbounds,
        notif_dut_temp_high_critic,
        notif_dut_co2_high,
        notif_dut_co2_high_endofday,
    ))
}

pub async fn update_notifs_dut(
    dev_info: &mut DevInfo,
    updated_dev_notifs: &Option<&Vec<Arc<NotifsCfgResponse_notif_item>>>,
    updated_dev_sched: Option<Arc<DutAutomationConfig>>,
    // O parâmetro "partial_update" indica se a lista "updated_dev_notifs" é completa ou parcial.
    // Se for parcial, vamos atualizar as notificações que estiverem na lista sem mexer nas outras.
    // Se for completa, vamos remover as notificações que não estiverem na lista.
    partial_update: bool,
) {
    let Some(updated_dev_notifs) = updated_dev_notifs else {
        if !partial_update {
            // Já que não tem nenhuma notificação monitorando o dispositivo, limpa a lista
            dev_info.has_notifs_dut.store(false, Ordering::Relaxed);
            *dev_info.notifs_dut.write().await = None;
        }
        return;
    };

    // Interpreta a resposta do API-Server (faz parse do JSON)
    let Some((
        mut notif_dut_temp_outofbounds,
        mut notif_dut_temp_high_critic,
        mut notif_dut_co2_high,
        mut notif_dut_co2_high_endofday,
    )) = parse_dut_notifs_list(updated_dev_notifs, &updated_dev_sched)
    else {
        if !partial_update {
            // Já que não tem nenhuma notificação monitorando o dispositivo, limpa a lista
            dev_info.has_notifs_dut.store(false, Ordering::Relaxed);
            *dev_info.notifs_dut.write().await = None;
        }
        return;
    };

    let schedule = updated_dev_sched.map(|x| x.schedule.clone());

    // Pega o "dev_info.notifs_dut" em modo "write" para atualizar
    let mut notifs_dut = dev_info.notifs_dut.write().await;

    if let Some(existente) = notifs_dut.as_mut() {
        existente.schedule = schedule;

        NotifDutTempOutOfBounds::update_new_notifs_parameters(
            &existente.notif_dut_temp_outofbounds,
            &mut notif_dut_temp_outofbounds,
        );

        NotifDutTempHighCritic::update_new_notifs_parameters(
            &existente.notif_dut_temp_high_critic,
            &mut notif_dut_temp_high_critic,
        );

        NotifDutCO2High::update_new_notifs_parameters(
            &existente.notif_dut_co2_high,
            &mut notif_dut_co2_high,
        );

        NotifDutCO2HighEndOfDay::update_new_notifs_parameters(
            &existente.notif_dut_co2_high_endofday,
            &mut notif_dut_co2_high_endofday,
        );

        if partial_update {
            for (notif_id, notif) in notif_dut_temp_outofbounds.into_iter() {
                existente.notif_dut_temp_outofbounds.insert(notif_id, notif);
            }
            for (notif_id, notif) in notif_dut_temp_high_critic.into_iter() {
                existente.notif_dut_temp_high_critic.insert(notif_id, notif);
            }
            for (notif_id, notif) in notif_dut_co2_high.into_iter() {
                existente.notif_dut_co2_high.insert(notif_id, notif);
            }
            for (notif_id, notif) in notif_dut_co2_high_endofday.into_iter() {
                existente
                    .notif_dut_co2_high_endofday
                    .insert(notif_id, notif);
            }
        } else {
            existente.notif_dut_temp_outofbounds = notif_dut_temp_outofbounds;
            existente.notif_dut_temp_high_critic = notif_dut_temp_high_critic;
            existente.notif_dut_co2_high = notif_dut_co2_high;
            existente.notif_dut_co2_high_endofday = notif_dut_co2_high_endofday;
        }
    } else {
        let mut notifs_dut_new = NotifsDut {
            schedule,
            notif_dut_temp_outofbounds,
            notif_dut_temp_high_critic,
            notif_dut_co2_high,
            notif_dut_co2_high_endofday,
            last_temperature: None,
            last_co2: None,
        };
        *notifs_dut = Some(notifs_dut_new);
    };

    dev_info.has_notifs_dut.store(true, Ordering::Relaxed);
}
