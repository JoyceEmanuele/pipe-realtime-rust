use super::notifications::dac::NotifsDac;
use super::notifications::dut::NotifsDut;
use super::notifications::send_queue::MsgToQueue;
use super::notifications::update_queue::MsgToQueueNotifUpdate;
use crate::ConfigFile;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::{collections::HashMap, sync::atomic::AtomicU64};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub struct GlobalVars {
    pub configfile: ConfigFile,
    pub devs_info: RwLock<HashMap<String, DevInfo>>,
    pub need_update_notifs: AtomicBool,
    pub to_notifs_queue: mpsc::Sender<MsgToQueue>,
    pub to_notif_update_queue: mpsc::Sender<MsgToQueueNotifUpdate>,
}

pub struct DevInfo {
    pub last_timestamp: AtomicU64, // Timestamp do servidor da última vez que chegou mensagem do dispostivo
    pub last_telemetry: RwLock<Option<DevLastMessage>>,
    pub has_notifs_dut: AtomicBool,
    pub notifs_dut: RwLock<Option<NotifsDut>>,
    pub has_notifs_dac: AtomicBool,
    pub notifs_dac: RwLock<Option<NotifsDac>>,
}

impl DevInfo {
    pub fn new(now_millis: u64, dev_id: &str) -> DevInfo {
        DevInfo {
            last_timestamp: AtomicU64::new(now_millis),
            last_telemetry: RwLock::new(None),
            notifs_dut: RwLock::new(None),
            has_notifs_dut: AtomicBool::new(false),
            notifs_dac: RwLock::new(None),
            has_notifs_dac: AtomicBool::new(false),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct DevLastMessage {
    pub telemetry: serde_json::Value, // último JSON que chegou em tópico 'data/...'
    pub ts: u64, // Timestamp do servidor da última vez que chegou mensagem do dispostivo
                 // pub topic?: TopicType // Tópico 'data/...' que foi usado, e não o tipo do dispositivo. O DMA por exemplo usa tópico de DUT.
                 // pub tsBefore: number // Timestamp do servidor da telemetria anterior à atual
}

impl GlobalVars {
    pub async fn new(
        configfile: ConfigFile,
    ) -> (
        GlobalVars,
        mpsc::Receiver<MsgToQueue>,
        mpsc::Receiver<MsgToQueueNotifUpdate>,
    ) {
        let (to_notifs_queue, receiver_notifs) = mpsc::channel::<MsgToQueue>(10000);
        let (to_notif_update_queue, receiver_notif_update) =
            mpsc::channel::<MsgToQueueNotifUpdate>(1000);

        let globs = GlobalVars {
            configfile,
            devs_info: RwLock::new(HashMap::new()),
            need_update_notifs: AtomicBool::new(true),
            to_notifs_queue,
            to_notif_update_queue,
        };

        (globs, receiver_notifs, receiver_notif_update)
    }
}
