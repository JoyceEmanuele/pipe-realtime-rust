mod helpers {
    pub mod lib_log;
    pub mod lib_http {
        pub mod buffer;
        pub mod protocol;
        pub mod request;
        pub mod response;
        pub mod service;
        pub mod types;
    }
    pub mod lib_essential_thread;
    pub mod lib_rumqtt;

    pub mod telemetry_payloads {
        pub mod parse_json_props;
        pub mod telemetry_formats;
    }
    pub mod envvars_loader;
    pub mod tls_socket_rustls;
}

mod app_realtime {
    pub mod configs;
    pub mod devs_cache;
    pub mod global_vars;
    pub mod http_router;
    pub mod mqtt_task;
    pub mod notifications;
    pub mod on_mqtt_message;
    pub mod endpoints {
        pub mod get_devices_last_telemetries;
        pub mod get_devices_last_ts;
        pub mod inspect_dev_notifications;
    }
}

use app_realtime::notifications::notifs_cfg;
use app_realtime::*;
use configs::ConfigFile;
use global_vars::GlobalVars;
use helpers::lib_log::{log_err, write_to_log_file, write_to_log_file_v2};
use helpers::*;
use std::sync::Arc;

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

static SERVICE_NAME: &str = "realtime";

/*
Ideia deste serviço:
 - Os outros serviços vão requisitar deste o status online/offline dos dispositivos.
 - Registra a última telemetria de cada dispositivo e o horário da última mensagem.
 - Este serviço pode também atualizar o banco de dados quando tem alteração de status online.
 - Precisa de uma estratégia para retirar da lista dispositivos removidos do Celsius.
*/

fn main() {
    // Criar pasta de logs e já inserir um registro indicando que iniciou o serviço
    lib_log::create_log_dir().expect("Não foi possível criar a pasta de logs");

    // Verifica se é só para testar o arquivo de config
    for arg in std::env::args().skip(1) {
        if arg == "--test-config" {
            envvars_loader::check_configfile();
            std::process::exit(0);
        }
    }

    crate::write_to_log_file("INIT", "Serviço iniciado");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Error creating tokio runtime");

    let result = rt.block_on(main2());

    println!("[EXIT] {:?}", result);
}

async fn main2() {
    let configfile = ConfigFile::from_env().expect("configfile inválido");
    let (globs, receiver_notifs, receiver_notifs_update) = GlobalVars::new(configfile).await;
    let globs = Arc::new(globs);

    // Inicia e aguarda as threads principais
    tokio::select! {
        // API HTTP oferecida para responder health-check e a última telemetria de cada dispositivo, por exemplo.
        result = tokio::spawn(
            lib_http::service::run_service_result(globs.configfile.listen_http_api.to_owned(), globs.clone(), &http_router::on_http_req)
        ) => { panic!("Some thread stopped: {:?}", result.unwrap()); },

        // Tarefa que fica buscando as mensagens MQTT do broker
        result = tokio::spawn(
            mqtt_task::task_mqtt_broker_reader(globs.clone())
        ) => { panic!("Some thread stopped: {:?}", result.unwrap()); },

        // Tarefa que salva no disco a última telemetria de cada dispositivo para conseguir recuperar quando reiniciar o realtime
        result = tokio::spawn(
            devs_cache::run_service(globs.clone())
        ) => { panic!("Some thread stopped: {:?}", result.unwrap()); },

        // Tarefa que mantém atualizado no realtime as notificações configuradas no API-Server
        result = tokio::spawn(
            notifs_cfg::run_service(globs.clone())
        ) => { panic!("Some thread stopped: {:?}", result.unwrap()); },

        // Tarefa que envia para o API-Server as notificações detectadas, fazendo até 3 tentativas por detecção.
        result = tokio::spawn(
            notifications::send_queue::start_queue_manager(receiver_notifs, globs.clone())
        ) => { panic!("Some thread stopped: {:?}", result.unwrap()); },

        // Recebe do API-Server avisos quando tem alterações nas notificações
        result = tokio::spawn(
            notifications::update_queue::start_update_queue_manager(receiver_notifs_update, globs.clone())
        ) => { panic!("Some thread stopped: {:?}", result.unwrap()); },
    }
}
