use super::registrar_deteccao;
use crate::app_realtime::global_vars::GlobalVars;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

pub type MsgToQueue = (&'static str, serde_json::Value);

pub async fn start_queue_manager(mut receiver: mpsc::Receiver<MsgToQueue>, globs: Arc<GlobalVars>) {
    loop {
        let (notif_path, notif_data) = receiver.recv().await.expect("Erro ao receber do mpsc");

        let mut tries = 0;
        loop {
            tries += 1;
            let result = registrar_deteccao(notif_path, &notif_data, &globs).await;
            if let Err(err) = result {
                crate::log_err("[216]", err);
                // TODO: criar uma estratégia para não bloquear a fila toda quando tiver erro em um endpoint específico
                // Outra melhoria é tratar diferentes tipos de erro que o API-Server pode retornar.
                // Por exemplo: se a notificação tiver sido excluída, nem precisa tentar de novo.
                tokio::time::sleep(Duration::from_secs(10)).await;
            } else {
                break;
            }
            if tries >= 3 {
                break;
            }
        }
    }
}
