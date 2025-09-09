use crate::app_realtime::global_vars::GlobalVars;
use serde_json::json;
use std::sync::{atomic::Ordering, Arc};

/**
 * Funções de debug das notificações
 * curl http://localhost:46136/diel-internal/realtime-rs/inspect_dev_notifications --data '{"device_code":"DEV123"}'
 */

pub async fn get_dev_notifs_info(
    device_code: &str,
    globs: &Arc<GlobalVars>,
) -> Result<String, String> {
    let devs_info = globs.devs_info.read().await;
    let dev_info = match devs_info.get(device_code) {
        None => {
            return Ok(format!(
                "Não existe registro do '{device_code}' no 'globs.devs_info'"
            ));
        }
        Some(x) => x,
    };

    let last_telemetry = dev_info
        .last_telemetry
        .read()
        .await
        .as_ref()
        .map(|x| x.telemetry.clone());

    // DUT
    let notifs_dut = dev_info.notifs_dut.read().await;
    let has_notifs_dut = dev_info.has_notifs_dut.load(Ordering::Relaxed);

    // DAC
    let notifs_dac = dev_info.notifs_dac.read().await;
    let has_notifs_dac = dev_info.has_notifs_dac.load(Ordering::Relaxed);

    let resposta = json!({
        "device_code": device_code,
        "last_telemetry": last_telemetry,

        "has_notifs_dut": has_notifs_dut,
        "notifs_dut": format!("{notifs_dut:?}"),
        "has_notifs_dac": has_notifs_dac,
        "notifs_dac": format!("{notifs_dac:?}"),
    });
    return Ok(resposta.to_string());
}
