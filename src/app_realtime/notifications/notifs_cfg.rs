/*
Neste arquivo ficam as funções de solicitar do API-Server as notificações cadastradas.
Aqui também fica o serviço que de tempo em tempo solicita novamente para verificar se teve alterações.
*/

use super::dac::notifs_dac;
use super::dut::notifs_dut;
use crate::app_realtime::global_vars::DevInfo;
use crate::app_realtime::{configs::ConfigFile, global_vars::GlobalVars};
use chrono::Datelike;
use chrono::Timelike;
use chrono::{NaiveDate, NaiveTime, TimeDelta, Weekday};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

/** Tarefa que busca no API-Server as configurações dos notificações e mantém atualizado no GlobalVars */
pub async fn run_service(globs: Arc<GlobalVars>) {
    let mut last_update: Option<std::time::Instant> = None;
    let mut need_update = false;
    loop {
        need_update = need_update || globs.need_update_notifs.load(Ordering::Relaxed);
        if !need_update {
            match last_update {
                Some(last_update) => {
                    // Se fizer mais de 1 hora que não atualiza, solicita atualização
                    need_update = last_update.elapsed() > Duration::from_secs(1 * 60 * 60);
                }
                None => {
                    // Se ainda não atualizou nenhum vez, solicita.
                    need_update = true;
                }
            }
        }
        if need_update {
            globs.need_update_notifs.store(false, Ordering::Relaxed);
            let result = update_all_notifs_configs(&globs).await;
            match result {
                Ok(()) => {
                    last_update = Some(std::time::Instant::now());
                    need_update = false;
                }
                Err(err) => {
                    crate::write_to_log_file("ERROR[197][notifs-cfg]", &err);
                }
            };
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn update_all_notifs_configs(globs: &Arc<GlobalVars>) -> Result<(), String> {
    let res = make_notifs_http_req(&globs.configfile, None).await?;
    let req_response = res.bytes().await.map_err(|err| format!("[56] {err}"))?;
    let req_response = std::str::from_utf8(&req_response).map_err(|e| e.to_string())?;

    let req_response =
        serde_json::from_str::<serde_json::Value>(req_response).map_err(|e| e.to_string())?;

    parse_complete_notifs_update(req_response, globs).await?;

    crate::write_to_log_file("info", "Update de notificações realizado");

    Ok(())
}

pub async fn update_specific_notifs(
    globs: &Arc<GlobalVars>,
    notifs_list: Vec<(u64, Option<Vec<String>>)>,
) -> Result<(), String> {
    let notif_ids: Vec<u64> = notifs_list.iter().map(|(notif_id, _)| *notif_id).collect();
    let res = make_notifs_http_req(&globs.configfile, Some(notif_ids)).await?;
    let req_response = res.bytes().await.map_err(|err| format!("[56] {err}"))?;
    let req_response = std::str::from_utf8(&req_response).map_err(|e| e.to_string())?;

    let req_response =
        serde_json::from_str::<serde_json::Value>(req_response).map_err(|e| e.to_string())?;

    parse_partial_notifs_update(req_response, globs, notifs_list).await?;

    crate::write_to_log_file("info", "Update de notificações realizado");

    Ok(())
}

async fn make_notifs_http_req(
    configfile: &ConfigFile,
    notif_ids: Option<Vec<u64>>,
) -> Result<reqwest::Response, String> {
    crate::write_to_log_file("info", "Solicitando update de notificações");
    let body = json!({
        "notif_ids": notif_ids,
    });

    let stats_url = format!(
        "{}/diel-internal/api-async/get-notifs-cfg-for-realtime",
        configfile.apiserver_internal_api
    );
    let client = reqwest::Client::new();
    let res = client
        .post(&stats_url)
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("[94] {err}"))?;
    let response_status = res.status();

    if response_status != reqwest::StatusCode::OK {
        let response_bytes = res.bytes().await.map_err(|err| format!("[98] {err}"))?;
        let packet_payload =
            std::str::from_utf8(&response_bytes).map_err(|err| format!("[99] {err}"))?;
        return Err(format!(
            "Invalid cfg_update response: {} {} {}",
            stats_url, response_status, packet_payload
        ));
    }

    Ok(res)
}

#[derive(Deserialize)]
struct NotifsCfgResponse {
    notifs_list: Vec<NotifsCfgResponse_notif_item>,
    devs_schedule: Vec<NotifsCfgResponse_sched_item>,
}
#[derive(Deserialize, Debug)]
pub struct NotifsCfgResponse_notif_item {
    #[serde(rename = "NOTIF_ID")]
    pub notif_id: u64,
    #[serde(rename = "COND_ID")]
    pub cond_id: String, // "${COND_VAR} ${COND_OP}" // { COND_VAR: string, COND_OP: string }
    #[serde(rename = "COND_PARS")]
    pub cond_pars: serde_json::Value, // { COND_VAL_1: string, COND_VAL_2: string }
    #[serde(rename = "DEV_IDS")]
    pub dev_ids: Vec<String>,
}
#[derive(Deserialize)]
pub struct NotifsCfgResponse_sched_item {
    pub dev_ids: Vec<String>,
    #[serde(rename = "TUSEMAX")]
    pub tusemax: Option<f64>,
    #[serde(rename = "TUSEMIN")]
    pub tusemin: Option<f64>,
    #[serde(rename = "CO2MAX")]
    pub co2max: Option<f64>,
    pub schedule: NotifsCfgResponse_schedule,
}

/*
   // A programação vem nesse formato:
   "schedule": {
       "by_day": {
           "mon":        { "permission": "allow", "start": "08:00", "end": "17:59" },
           "2025-12-31": { "permission": "allow", "start": "00:00", "end": "23:59" }
       }
   }
*/
#[derive(Deserialize)]
struct NotifsCfgResponse_schedule {
    pub by_day: HashMap<String, NotifsCfgResponse_schedule_byday>,
}
#[derive(Deserialize)]
struct NotifsCfgResponse_schedule_byday {
    pub permission: String, // "allow" | "forbid"
    pub start: String,      // "23:59"
    pub end: String,        // "23:59"
}

#[derive(Debug)]
pub struct DutAutomationConfig {
    pub tusemax: Option<f64>,
    pub tusemin: Option<f64>,
    pub co2max: Option<f64>,
    pub schedule: Arc<AutomationSchedule>,
}

pub async fn parse_partial_notifs_update(
    parsed: serde_json::Value,
    globs: &Arc<GlobalVars>,
    changed_notifs_list: Vec<(u64, Option<Vec<String>>)>,
) -> Result<(), String> {
    // Faz parse da resposta JSON que vem do API-Server
    let parsed: NotifsCfgResponse =
        serde_json::from_value(parsed).map_err(|err| format!("[72] {err}"))?;

    // Interpreta a programação dos DUTs
    // "aut_cfg_by_dev" faz associação de "dev_id" com os parâmetros de automação de DUT (DutAutomationConfig)
    let aut_cfg_by_dev = get_devices_automation_params(parsed.devs_schedule);

    // Interpreta a lista de notificações
    // "notifs_by_dev" faz associação de "dev_id" com a lista de todas as notificações monitorando ele
    let notifs_by_dev = get_notifs_by_each_device(parsed.notifs_list);

    let mut dev_info = globs.devs_info.write().await;

    for (notif_id, removed_dev_ids) in changed_notifs_list.iter() {
        let Some(removed_dev_ids) = removed_dev_ids else {
            continue;
        };
        for dev_id in removed_dev_ids {
            let Some(dev_info) = dev_info.get_mut(dev_id) else {
                continue;
            };
            if let Some(notifs_dac) = dev_info.notifs_dac.write().await.as_mut() {
                notifs_dac.remove_notif_id(*notif_id);
            }
            if let Some(notifs_dut) = dev_info.notifs_dut.write().await.as_mut() {
                notifs_dut.remove_notif_id(*notif_id);
            }
        }
    }

    for (dev_id, updated_dev_notifs) in notifs_by_dev.iter() {
        let Some(dev_info) = dev_info.get_mut(dev_id) else {
            continue;
        };

        // Programação associada ao dispositivo
        let updated_dev_sched = aut_cfg_by_dev.get(dev_id).map(|x| x.clone());

        notifs_dut::update_notifs_dut(dev_info, &Some(updated_dev_notifs), updated_dev_sched, true)
            .await;

        notifs_dac::update_notifs_dac(dev_info, &Some(updated_dev_notifs), true).await;
    }

    return Ok(());
}

pub async fn parse_complete_notifs_update(
    parsed: serde_json::Value,
    globs: &Arc<GlobalVars>,
) -> Result<(), String> {
    // Faz parse da resposta JSON que vem do API-Server
    let parsed: NotifsCfgResponse =
        serde_json::from_value(parsed).map_err(|err| format!("[72] {err}"))?;

    // Interpreta a programação dos DUTs
    // "aut_cfg_by_dev" faz associação de "dev_id" com os parâmetros de automação de DUT (DutAutomationConfig)
    let aut_cfg_by_dev = get_devices_automation_params(parsed.devs_schedule);

    // Interpreta a lista de notificações
    // "notifs_by_dev" faz associação de "dev_id" com a lista de todas as notificações monitorando ele
    let notifs_by_dev = get_notifs_by_each_device(parsed.notifs_list);

    // Atualiza o globs.devs_info com as novas configurações
    let mut devs_info = globs.devs_info.write().await;
    for (dev_id, dev_info) in devs_info.iter_mut() {
        // Lista de todas as notificações monitorando este dispositivo
        let device_full_notif_list = notifs_by_dev.get(dev_id);

        // Programação associada ao dispositivo
        let updated_dev_sched = aut_cfg_by_dev.get(dev_id).map(|x| x.clone());

        // Ajusta no "dev_info" (do "globs") a lista de notificações associadas ao dispositivo
        update_all_device_notifs(dev_info, &device_full_notif_list, updated_dev_sched).await;
    }

    return Ok(());
}

fn get_devices_automation_params(
    devs_schedule: Vec<NotifsCfgResponse_sched_item>,
) -> HashMap<String, Arc<DutAutomationConfig>> {
    let mut aut_cfg_by_dev: HashMap<String, Arc<DutAutomationConfig>> = HashMap::new();
    for new_sched in devs_schedule.into_iter() {
        // Faz parse da programação
        let parsed_sched = match parse_cfg_schedule(new_sched.schedule) {
            Ok(x) => x,
            Err(_err) => {
                crate::write_to_log_file("ERROR", "[230] Invalid schedule");
                continue;
            }
        };
        let autom_cfg = Arc::new(DutAutomationConfig {
            tusemax: new_sched.tusemax,
            tusemin: new_sched.tusemin,
            co2max: new_sched.co2max,
            schedule: Arc::new(parsed_sched),
        });

        // Associa a programação aos dispositivos que seguem ela
        for dev_id in new_sched.dev_ids.iter() {
            aut_cfg_by_dev.insert(dev_id.to_owned(), autom_cfg.clone());
        }
    }
    aut_cfg_by_dev
}

fn get_notifs_by_each_device(
    notifs_list: Vec<NotifsCfgResponse_notif_item>,
) -> HashMap<String, Vec<Arc<NotifsCfgResponse_notif_item>>> {
    let mut notifs_by_dev: HashMap<String, Vec<Arc<NotifsCfgResponse_notif_item>>> = HashMap::new();
    for notif in notifs_list.into_iter() {
        let notif = Arc::new(notif);
        for dev_id in notif.dev_ids.iter() {
            match notifs_by_dev.get_mut(dev_id) {
                Some(list) => {
                    list.push(notif.clone());
                }
                None => {
                    notifs_by_dev.insert(dev_id.to_owned(), vec![notif.clone()]);
                }
            };
        }
    }
    notifs_by_dev
}

async fn update_all_device_notifs(
    dev_info: &mut DevInfo,
    updated_dev_notifs: &Option<&Vec<Arc<NotifsCfgResponse_notif_item>>>,
    updated_dev_sched: Option<Arc<DutAutomationConfig>>,
) {
    // A função "update_notifs_dut" vai atualizar o "dev_info.notifs_dut" com os dados de "dut_notifs"
    notifs_dut::update_notifs_dut(dev_info, updated_dev_notifs, updated_dev_sched, false).await;

    // A função "update_notifs_dut" vai atualizar o "dev_info.notifs_dut" com os dados de "dut_notifs"
    notifs_dac::update_notifs_dac(dev_info, updated_dev_notifs, false).await;
}

fn parse_cfg_schedule(schedule: NotifsCfgResponse_schedule) -> Result<AutomationSchedule, String> {
    let mut parsed_prog = AutomationSchedule {
        by_day: HashMap::new(),
    };
    for (day, day_prog) in schedule.by_day {
        let permission = match day_prog.permission.as_str() {
            "allow" => ProgPermission::Allow,
            "forbid" => ProgPermission::Forbid,
            permission => {
                return Err(format!("Incompatible programming permission: {permission}"));
            }
        };
        let start = NaiveTime::parse_from_str(&day_prog.start, "%H:%M")
            .map_err(|err| format!("[273] {err}"))?;
        let (end, _) = NaiveTime::parse_from_str(&day_prog.end, "%H:%M")
            .map_err(|err| format!("[273] {err}"))?
            .overflowing_add_signed(TimeDelta::seconds(59));
        // Os 59 segundos da linha acima são adicionados pois a programação é definida no formato "00:00 - 23:59"
        // Como os timestamps das telemetrias têm resolução de 1 segundo, o exemplo acima equivale a "00:00:00 - 23:59:59"
        parsed_prog.by_day.insert(
            day,
            DayProg {
                permission,
                start,
                end,
            },
        );
    }
    Ok(parsed_prog)
}

#[derive(Debug)]
pub enum ProgPermission {
    Allow,
    Forbid,
    Ventilation,
}

#[derive(Debug)]
pub struct DayProg {
    pub permission: ProgPermission,
    pub start: NaiveTime, // '00:00'
    pub end: NaiveTime,   // '23:59'
}
impl DayProg {
    pub fn is_inside_sched(&self, time: NaiveTime) -> bool {
        let time_index = time.num_seconds_from_midnight();
        let is_between = (time_index >= self.start.num_seconds_from_midnight())
            && (time_index <= self.end.num_seconds_from_midnight());

        match self.permission {
            ProgPermission::Allow => is_between,
            ProgPermission::Forbid => !is_between,
            ProgPermission::Ventilation => false,
        }
    }
}

#[derive(Debug)]
pub struct AutomationSchedule {
    pub by_day: HashMap<String, DayProg>, // 'mon', 'tue', '2024-12-25'
}
impl AutomationSchedule {
    pub fn get_for(&self, day: &NaiveDate) -> Option<&DayProg> {
        let day_str = day.format("%Y-%m-%d").to_string();
        if let Some(prog) = self.by_day.get(&day_str) {
            return Some(prog);
        }
        let week_day = match day.weekday() {
            Weekday::Mon => "mon",
            Weekday::Tue => "tue",
            Weekday::Wed => "wed",
            Weekday::Thu => "thu",
            Weekday::Fri => "fri",
            Weekday::Sat => "sat",
            Weekday::Sun => "sun",
        };
        if let Some(prog) = self.by_day.get(week_day) {
            return Some(prog);
        }
        return None;
    }
}
