use super::dac_l1::dac_compressor_usage_hours::{
    NotifCompressorUsedAfterHour, NotifCompressorUsedBeforeHour,
};
use super::dac_l1::LastDacL1;
use crate::app_realtime::global_vars::DevInfo;
use crate::app_realtime::notifications::notifs_cfg::NotifsCfgResponse_notif_item;
use std::sync::atomic::Ordering;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct NotifsDac {
    pub notif_compressor_used_before_time: HashMap<u64, NotifCompressorUsedBeforeHour>,
    pub notif_compressor_used_after_time: HashMap<u64, NotifCompressorUsedAfterHour>,
    pub last_l1: Option<LastDacL1>,
}
impl NotifsDac {
    pub fn remove_notif_id(&mut self, notif_id: u64) {
        self.notif_compressor_used_before_time.remove(&notif_id);
        self.notif_compressor_used_after_time.remove(&notif_id);
    }
}

fn parse_dac_notifs_list(
    updated_dev_notifs: &Vec<Arc<NotifsCfgResponse_notif_item>>,
) -> Option<(
    HashMap<u64, NotifCompressorUsedBeforeHour>,
    HashMap<u64, NotifCompressorUsedAfterHour>,
)> {
    let mut notif_compressor_used_before_time = HashMap::new();
    let mut notif_compressor_used_after_time = HashMap::new();

    for notif in updated_dev_notifs.iter() {
        match notif.cond_id.as_str() {
            "COMP_TIME <" => {
                match NotifCompressorUsedBeforeHour::from_notif_cfg(notif) {
                    Ok(notif) => {
                        notif_compressor_used_before_time.insert(notif.notif_id, notif);
                    }
                    Err(_err) => {} // Ignora silenciosamente
                };
            }
            "COMP_TIME >" => {
                match NotifCompressorUsedAfterHour::from_notif_cfg(notif) {
                    Ok(notif) => {
                        notif_compressor_used_after_time.insert(notif.notif_id, notif);
                    }
                    Err(_err) => {} // Ignora silenciosamente
                };
            }
            _ => {
                // Ignore
            }
        }
    }

    let is_empty =
        notif_compressor_used_before_time.is_empty() && notif_compressor_used_after_time.is_empty();

    if is_empty {
        return None;
    }

    Some((
        notif_compressor_used_before_time,
        notif_compressor_used_after_time,
    ))
}

pub async fn update_notifs_dac(
    dev_info: &mut DevInfo,
    updated_dev_notifs: &Option<&Vec<Arc<NotifsCfgResponse_notif_item>>>,
    // O parâmetro "partial_update" indica se a lista "updated_dev_notifs" é completa ou parcial.
    // Se for parcial, vamos atualizar as notificações que estiverem na lista sem mexer nas outras.
    // Se for completa, vamos remover as notificações que não estiverem na lista.
    partial_update: bool,
) {
    let Some(updated_dev_notifs) = updated_dev_notifs else {
        if !partial_update {
            // Já que não tem nenhuma notificação monitorando o dispositivo, limpa a lista
            dev_info.has_notifs_dac.store(false, Ordering::Relaxed);
            *dev_info.notifs_dac.write().await = None;
        }
        return;
    };

    // Interpreta a resposta do API-Server (faz parse do JSON)
    let Some((mut notif_compressor_used_before_time, mut notif_compressor_used_after_time)) =
        parse_dac_notifs_list(updated_dev_notifs)
    else {
        if !partial_update {
            // Já que não tem nenhuma notificação monitorando o dispositivo, limpa a lista
            dev_info.has_notifs_dac.store(false, Ordering::Relaxed);
            *dev_info.notifs_dac.write().await = None;
        }
        return;
    };

    // Pega o "dev_info.notifs_dac" em modo "write" para atualizar
    let mut notifs_dac = dev_info.notifs_dac.write().await;

    if let Some(existente) = notifs_dac.as_mut() {
        NotifCompressorUsedBeforeHour::update_new_notifs_parameters(
            &existente.notif_compressor_used_before_time,
            &mut notif_compressor_used_before_time,
        );

        NotifCompressorUsedAfterHour::update_new_notifs_parameters(
            &existente.notif_compressor_used_after_time,
            &mut notif_compressor_used_after_time,
        );

        if partial_update {
            for (notif_id, notif) in notif_compressor_used_before_time.into_iter() {
                existente
                    .notif_compressor_used_before_time
                    .insert(notif_id, notif);
            }
            for (notif_id, notif) in notif_compressor_used_after_time.into_iter() {
                existente
                    .notif_compressor_used_after_time
                    .insert(notif_id, notif);
            }
        } else {
            existente.notif_compressor_used_before_time = notif_compressor_used_before_time;
            existente.notif_compressor_used_after_time = notif_compressor_used_after_time;
        }
    } else {
        let notifs_dac_new = NotifsDac {
            notif_compressor_used_before_time,
            notif_compressor_used_after_time,
            last_l1: None,
        };
        *notifs_dac = Some(notifs_dac_new);
    };

    dev_info.has_notifs_dac.store(true, Ordering::Relaxed);
}
