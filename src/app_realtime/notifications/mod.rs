use super::global_vars::{DevInfo, GlobalVars};
use chrono::{NaiveDateTime, TimeDelta};
use std::sync::{atomic::Ordering, Arc};

pub mod dac;
pub mod dut;
pub mod inspection;
pub mod notifs_cfg;
pub mod send_queue;
pub mod update_queue;

pub async fn on_device_telemetry(
    payload_json: &serde_json::Value,
    dev_id: &str,
    dev_info: &DevInfo,
    globs: &Arc<GlobalVars>,
) {
    // Confere as notificações de DUT
    if dev_info.has_notifs_dut.load(Ordering::Relaxed) == true {
        let mut notifs_dut = dev_info.notifs_dut.write().await;
        if let Some(notifs_dut) = notifs_dut.as_mut() {
            dut::on_dut_telemetry(&payload_json, notifs_dut, &dev_id, &globs).await;
        };
    }

    // Confere as notificações de DAC
    if dev_info.has_notifs_dac.load(Ordering::Relaxed) == true {
        let mut notifs_dac = dev_info.notifs_dac.write().await;
        if let Some(notifs_dac) = notifs_dac.as_mut() {
            dac::on_dac_telemetry(&payload_json, notifs_dac, &dev_id, &globs).await;
        };
    }
}

/// Faz uma requisição HTTP para o API-Server informando a detecção da notificação.
/// O API-Server é que vai montar o email de notificação e enviar.
pub async fn registrar_deteccao(
    notif_path: &'static str,
    detection: &serde_json::Value,
    globs: &Arc<GlobalVars>,
) -> Result<(), String> {
    crate::write_to_log_file("NOTIF-DETECTED", &detection.to_string());
    let body = detection; // serde_json::to_value(detection).map_err(|err| format!("[386] {err}"))?;

    let api_url = format!(
        "{}/diel-internal/api-async/notification-detected{notif_path}", // notif_path = "/DUT_T/AcimaLimiteCritica"
        globs.configfile.apiserver_internal_api
    );
    let client = reqwest::Client::new();
    let res = client
        .post(&api_url)
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("[398] {err}"))?;
    let response_status = res.status();

    if response_status != reqwest::StatusCode::OK {
        let response_bytes = res.bytes().await.map_err(|e| e.to_string())?;
        let packet_payload = std::str::from_utf8(&response_bytes).map_err(|e| e.to_string())?;
        return Err(format!(
            "Invalid api-server response: {api_url} {response_status} {packet_payload}"
        ));
    }

    Ok(())
}

/// Esta função calcula o tempo desde a última telemetria recebida deste mesmo DUT e retorna 3 valores:
///  - delta_secs: u64 => o número de segundos desde o último valor recebido
///  - descontinuidade: bool => indica se o último valor recebido foi recente ou se houve um intervalo sem informações
///  - new_day: bool => indica se trocou o dia considerando o timestamp da telemetria sem conversão para UTC
pub fn get_telemetry_delta(
    curr_ts: &NaiveDateTime,
    prev_ts: Option<&NaiveDateTime>,
    // curr_ts: &DateTime<FixedOffset>,
    // prev_ts: Option<&DateTime<FixedOffset>>,
    continuity_seconds: u64, // Se o intervalo entre leituras for muito grande o delta será limitado a este valor
) -> (u64, bool, bool) {
    let mut delta: Option<TimeDelta> = None;
    let mut new_day = true;
    if let Some(prev_ts) = prev_ts {
        let prev_telemetry_day = prev_ts.date(); // _naive();
        let curr_telemetry_day = curr_ts.date(); // _naive();
        let same_day = curr_telemetry_day == prev_telemetry_day;
        if same_day {
            new_day = false;
        }
        let dur = curr_ts.signed_duration_since(*prev_ts);
        delta = Some(dur);
    }

    // A telemetria de agora tem que ser depois da última, se tiver timestamp anterior tem que descartar ou resetar os contadores.
    let mut descontinuidade = false;
    let delta_secs = if let Some(delta) = delta {
        let mut delta_secs = delta.num_seconds();
        if delta_secs > (continuity_seconds as i64) {
            delta_secs = continuity_seconds as i64;
            descontinuidade = true;
        }
        if delta_secs < 0 {
            // if (!(delta > 0)) delta = 0;
            delta_secs = 0;
            descontinuidade = true;
        }
        delta_secs
    } else {
        descontinuidade = true;
        0
    };

    (delta_secs as u64, descontinuidade, new_day)
}
