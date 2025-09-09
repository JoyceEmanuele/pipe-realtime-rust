/**
 * Funções de debug das notificações
 */
use crate::app_realtime::notifications::inspection::get_dev_notifs_info;
use crate::global_vars::GlobalVars;
use crate::helpers::lib_http::response::respond_http_json_bytes;
use crate::lib_http::types::{HttpRequest, HttpResponse};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ParamsInspectDevNotifs {
    pub device_code: String,
}
pub async fn inspect_dev_notifications(
    req: &HttpRequest,
    globs: &Arc<GlobalVars>,
) -> Result<HttpResponse, String> {
    let req_params: ParamsInspectDevNotifs =
        serde_json::from_slice(&req.content).map_err(|e| e.to_string())?;

    let response = get_dev_notifs_info(&req_params.device_code, globs)
        .await
        .map_err(|err| format!("[58] {err}"))?;

    Ok(respond_http_json_bytes(501, response.into_bytes()))
}
