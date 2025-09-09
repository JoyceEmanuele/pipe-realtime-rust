use super::endpoints::get_devices_last_telemetries::get_devices_last_telemetries;
use super::endpoints::get_devices_last_ts::get_devices_last_ts;
use super::endpoints::inspect_dev_notifications::inspect_dev_notifications;
use crate::lib_http::response::{respond_http_plain_text, send_response};
use crate::lib_http::types::HttpRequest;
use crate::GlobalVars;
use std::sync::Arc;
use tokio::net::TcpStream;

pub async fn on_http_req(
    req: HttpRequest,
    is_internal: bool,
    mut socket: TcpStream,
    globs: Arc<GlobalVars>,
) {
    let response = match &req.path[..] {
        "/diel-internal/realtime-rs/getDevicesLastTelemetries" => {
            get_devices_last_telemetries(&req, &globs)
                .await
                .unwrap_or_else(|err| respond_http_plain_text(400, &err))
        }
        "/diel-internal/realtime-rs/getDevicesLastTS" => get_devices_last_ts(&req, &globs)
            .await
            .unwrap_or_else(|err| respond_http_plain_text(400, &err)),
        "/diel-internal/realtime-rs/inspect_dev_notifications" => {
            inspect_dev_notifications(&req, &globs)
                .await
                .unwrap_or_else(|err| respond_http_plain_text(400, &err))
        }
        _ => {
            crate::write_to_log_file(
                "ERROR",
                &format!("Invalid request: {} {}", req.method, req.path),
            );
            respond_http_plain_text(404, &(String::new() + "Not found: " + &req.path))
        }
    };
    // let response = respond_http_plain_text(500, &err);
    if let Err(err) = {
        send_response(&mut socket, &response).await // socket_write
    } {
        crate::write_to_log_file("ERROR[81]", &err.to_string());
    }
}
