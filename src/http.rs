use core::pin::pin;
use defmt::info;
use embassy_net::Stack;
use embassy_time::{Duration, Instant, Timer};
use heapless::Vec;
use nanofish::{
    DefaultHttpServer, HttpHandler, HttpRequest, HttpResponse, ResponseBody, StatusCode,
};

#[derive(Debug)]
pub struct HttpServer;

impl HttpHandler for HttpServer {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
        let start = Instant::now();
        let response = match request.path {
            "/healthz" => HttpResponse {
                status_code: StatusCode::Ok,
                headers: Vec::new(),
                body: ResponseBody::Text("{\"status\":\"ok\"}"),
            },
            "/pausez" => {
                Timer::after(Duration::from_millis(1000)).await;
                HttpResponse {
                    status_code: StatusCode::Ok,
                    headers: Vec::new(),
                    body: ResponseBody::Text("OK"),
                }
            }
            _ => HttpResponse {
                status_code: StatusCode::NotFound,
                headers: Vec::new(),
                body: ResponseBody::Text("Not Found"),
            },
        };

        let duration = Instant::now() - start;
        info!(
            "[HTTP] \"{} {} {}\" {} {} {} {}.{:03}",
            request.method.as_str(),
            request.path,
            request.version,
            response.status_code.as_u16(),
            request.body.len(),
            response.body.len(),
            duration.as_micros() / 1000,
            duration.as_micros() % 1000
        );

        Ok(response)
    }
}

#[embassy_executor::task]
pub async fn run_http_server(stack: Stack<'static>) {
    let mut server = DefaultHttpServer::new(80); // Listen on port 80
    let handler = HttpServer;

    info!("[HTTP] Serving http");
    pin!(server.serve(stack, handler)).await;
}
