use core::pin::pin;
use defmt::info;
use embassy_net::Stack;
use heapless::Vec;
use nanofish::{
    DefaultHttpServer, HttpHandler, HttpRequest, HttpResponse, ResponseBody, StatusCode,
};

#[derive(Debug)]
pub struct PromHandler;

impl HttpHandler for PromHandler {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
        let response = match request.path {
            "/healthz" => HttpResponse {
                status_code: StatusCode::Ok,
                headers: Vec::new(),
                body: ResponseBody::Text("{\"status\":\"ok\"}"),
            },
            _ => HttpResponse {
                status_code: StatusCode::NotFound,
                headers: Vec::new(),
                body: ResponseBody::Text("Not Found"),
            },
        };

        info!(
            "[HTTP] {} {} {} -> {} {}",
            request.method.as_str(),
            request.path,
            request.body.len(),
            response.status_code.as_u16(),
            response.body.len()
        );

        Ok(response)
    }
}

#[embassy_executor::task]
pub async fn run_http_server(stack: Stack<'static>) {
    let mut server = DefaultHttpServer::new(80); // Listen on port 80
    let handler = PromHandler;

    info!("[HTTP] Serving http");
    pin!(server.serve(stack, handler)).await;
}
