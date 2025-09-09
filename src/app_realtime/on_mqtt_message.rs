use super::global_vars::{DevInfo, DevLastMessage};
use super::notifications;
use crate::GlobalVars;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/*

function onDeviceMessage(devId: string) {
  let devLastMessages = lastMessages[devId];
  const prevTS = devLastMessages?.ts;

  if (devLastMessages) {
    devLastMessages.ts = Date.now();
  } else {
    devLastMessages = lastMessages[devId] = {
      tsBefore: null,
      ts: Date.now(),
    };
  }
  deviceLastTs[devId] = devLastMessages.ts;

  const becameOnline = !!(prevTS && ((devLastMessages.ts - prevTS) > TIMEOUT_LATE));
  const wasOffline = !!(prevTS && ((devLastMessages.ts - prevTS) > TIMEOUT_OFFLINE));
  const tsBefore = prevTS || null;
  if (becameOnline) {
    // Avisa o front através do websocket
    listenerForStatusChange?.(devId, 'ONLINE');
  }

  return { devLastMessages, wasOffline, tsBefore };
}

*/

pub fn process_payload(packet: rumqttc::Publish, globs: &Arc<GlobalVars>) {
    let mut topic = packet.topic.as_str();

    if topic.starts_with("apiserver/") {
        process_payload_from_apiserver(packet, globs);
        return;
    }

    if topic.starts_with("iotrelay/") {
        topic = &topic[9..];
    }

    let is_data = topic.starts_with("data/");

    // Só tratamos mensagens nos tópicos de "data" e "control"
    let is_valid_topic = is_data || topic.starts_with("control/");
    if !is_valid_topic {
        // Ignore
        return;
    }

    tokio::spawn(process_payload_on_valid_topic(
        globs.clone(),
        packet,
        is_data,
    ));
}

async fn process_payload_on_valid_topic(
    globs: Arc<GlobalVars>,
    packet: rumqttc::Publish,
    is_data: bool,
) {
    // Faz parse do pacote MQTT como JSON
    let (_payload_str, payload_json, dev_id) = match parse_payload_json(&packet) {
        ResultJsonParse::Ok(x) => x,
        ResultJsonParse::Ignore => {
            // Provavelmente é um JSON inválido mas deve ser ignorado sem colocar no log como erro
            return;
        }
        ResultJsonParse::Err(err) => {
            let message = format!("[76] {err}");
            crate::write_to_log_file_v2("ERROR", &message, false);
            return;
        }
    };

    // Timestamp atual no servidor da última mensagem que chegou do dispositivo
    let now_millis: u64 = {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .try_into()
            .expect("timestamp too large")
    };

    // Pega no globs.devs_info as informações do dispositivo
    let mut devs_info = globs.devs_info.read().await;
    let mut dev_info_opt = devs_info.get(&dev_id);
    if dev_info_opt.is_none() {
        // Se ainda não tiver registro no globs.devs_info, cria um novo e insere
        drop(devs_info); // Este drop é só para garantir que não vai dar deadlock no globs.devs_info
        let new_dev_info = DevInfo::new(now_millis, &dev_id);
        globs
            .devs_info
            .write()
            .await
            .insert(dev_id.to_owned(), new_dev_info);
        devs_info = globs.devs_info.read().await;
        dev_info_opt = devs_info.get(&dev_id);
    };
    let Some(dev_info) = dev_info_opt else {
        return;
    };

    // Atualiza o last_timestamp e o last_telemetry
    atualizar_dev_info(dev_info, &payload_json, is_data, now_millis).await;

    // Confere as notificações
    notifications::on_device_telemetry(&payload_json, &dev_id, dev_info, &globs).await;
}

async fn atualizar_dev_info(
    dev_info: &DevInfo,
    payload_json: &serde_json::Value,
    is_telemetry: bool,
    now_millis: u64,
) {
    // Atualiza o last_timestamp
    dev_info.last_timestamp.store(now_millis, Ordering::Relaxed);

    // Atualiza o last_telemetry
    if is_telemetry {
        let mut last_telemetry = dev_info.last_telemetry.write().await;
        if let Some(last_telemetry) = last_telemetry.as_mut() {
            last_telemetry.ts = now_millis;
            last_telemetry.telemetry = payload_json.clone();
        } else {
            *last_telemetry = Some(DevLastMessage {
                ts: now_millis,
                telemetry: payload_json.clone(),
            });
        }
    }
}

enum ResultJsonParse<T> {
    Ok(T),
    Err(String),
    Ignore,
}

fn parse_payload_json<'a>(
    packet: &'a rumqttc::Publish,
) -> ResultJsonParse<(&'a str, serde_json::Value, String)> {
    let payload_str = match std::str::from_utf8(&packet.payload) {
        Ok(v) => v,
        Err(err) => {
            return ResultJsonParse::Err(format!("Invalid payload: {}", err));
        }
    };

    // Ignore invalid payload
    if !payload_str.starts_with('{') {
        // For example: "Current RMT state:..."
        return ResultJsonParse::Ignore;
    }

    // Parse payload string to JSON object
    let payload_json: serde_json::Value = match serde_json::from_str(payload_str) {
        Ok(v) => v,
        Err(err) => {
            return ResultJsonParse::Err(format!(
                "Invalid payload [197]: {}\n  {}",
                err, payload_str
            ));
        }
    };

    let dev_id = match payload_json["dev_id"].as_str() {
        Some(v) => v.to_owned(),
        None => {
            return ResultJsonParse::Err(format!("Invalid payload [208]: {}", payload_str));
        }
    };

    ResultJsonParse::Ok((payload_str, payload_json, dev_id))
}

fn parse_apiserver_message<'a>(
    packet: &'a rumqttc::Publish,
) -> ResultJsonParse<(&'a str, serde_json::Value)> {
    let payload_str = match std::str::from_utf8(&packet.payload) {
        Ok(v) => v,
        Err(err) => {
            return ResultJsonParse::Err(format!("Invalid payload: {}", err));
        }
    };

    // Parse payload string to JSON object
    let payload_json: serde_json::Value = match serde_json::from_str(payload_str) {
        Ok(v) => v,
        Err(err) => {
            return ResultJsonParse::Err(format!(
                "Invalid payload [197]: {}\n  {}",
                err, payload_str
            ));
        }
    };

    ResultJsonParse::Ok((payload_str, payload_json))
}

fn process_payload_from_apiserver(packet: rumqttc::Publish, globs: &Arc<GlobalVars>) {
    // Por enquanto o realtime só processa mensagens deste tópico:
    if packet.topic != "apiserver/notif-change" {
        return;
    }

    // Faz parse do pacote MQTT como JSON
    let (_, payload_json) = match parse_apiserver_message(&packet) {
        ResultJsonParse::Ok(x) => x,
        ResultJsonParse::Ignore => {
            return;
        }
        ResultJsonParse::Err(err) => {
            let message = format!("[77] {err}");
            crate::write_to_log_file_v2("ERROR", &message, false);
            return;
        }
    };

    if packet.topic == "apiserver/notif-change" {
        // Houve mudança de config de notificações, tem que atualizar
        on_notif_change(globs, &payload_json);
    }
}

fn on_notif_change(globs: &Arc<GlobalVars>, payload_json: &serde_json::Value) {
    let notif_id = match payload_json["notif_id"].as_u64() {
        Some(v) => v.to_owned(),
        None => {
            let message = format!("[77] Invalid payload: {}", payload_json.to_string());
            crate::write_to_log_file_v2("ERROR", &message, false);
            return;
        }
    };

    let mut removed_dev_ids: Option<Vec<String>> = None;
    if let Some(list) = payload_json["removed_dev_ids"].as_array() {
        let mut new_list = Vec::with_capacity(list.len());
        for item in list.iter() {
            if let Some(dev_id) = item.as_str() {
                new_list.push(dev_id.to_owned());
            }
        }
        if !new_list.is_empty() {
            removed_dev_ids = Some(new_list);
        }
    };

    if globs.to_notif_update_queue.capacity() > 0 {
        let globs = globs.clone();
        tokio::spawn(async move {
            let result = globs
                .to_notif_update_queue
                .send((notif_id, removed_dev_ids))
                .await;
            if let Err(err) = result {
                crate::write_to_log_file("ERROR", &format!("[271] {err}"));
            }
        });
    }
}
