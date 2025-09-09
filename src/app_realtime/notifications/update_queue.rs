use super::notifs_cfg::update_specific_notifs;
use crate::app_realtime::global_vars::GlobalVars;
use std::sync::Arc;
use tokio::sync::mpsc;

pub type MsgToQueueNotifUpdate = (u64, Option<Vec<String>>);

pub async fn start_update_queue_manager(
    mut receiver: mpsc::Receiver<MsgToQueueNotifUpdate>,
    globs: Arc<GlobalVars>,
) {
    loop {
        let mut list = Vec::new();
        receiver.recv_many(&mut list, 20).await;

        if list.is_empty() {
            crate::log_err("[217]", "Erro ao ler MsgToQueueNotifUpdate");
            break;
        }

        let result = update_specific_notifs(&globs, list).await;
        if let Err(err) = result {
            crate::log_err("[214]", err);
        }
    }
}
