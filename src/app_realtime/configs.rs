use crate::envvars_loader;
use crate::lib_rumqtt::BrokerConfig;

pub struct ConfigFile {
    pub listen_http_api: String,
    pub broker_config: BrokerConfig,
    pub apiserver_internal_api: String,
}

impl ConfigFile {
    pub fn from_env() -> Result<ConfigFile, String> {
        envvars_loader::load_env_vars();

        let broker_config = BrokerConfig {
            host: envvars_loader::get_var_string_required("brokerConfig_host")?,
            port: envvars_loader::get_var_u16_required("brokerConfig_port")?,
            username: envvars_loader::get_var_string_required("brokerConfig_username")?,
            password: envvars_loader::get_var_string_required("brokerConfig_password")?,
            use_tls: false,
            ca_cert: None,
        };

        Ok(ConfigFile {
            listen_http_api: envvars_loader::get_var_string_required("listen_http_api_realtime")?,
            broker_config,
            apiserver_internal_api: envvars_loader::get_var_string_required(
                "APISERVER_INTERNAL_API",
            )?,
        })
    }
}
