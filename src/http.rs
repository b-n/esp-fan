use core::pin::pin;
use defmt::info;
use embassy_net::Stack;
use embassy_time::{Duration, Instant, Timer};
use heapless::{String, Vec};
use nanofish::{
    DefaultHttpServer, HttpHandler, HttpMethod, HttpRequest, HttpResponse, ResponseBody, StatusCode,
};

use crate::prom::METRICS;

const RESPONSE_BUFFER_SIZE: usize = 16383;

#[derive(Debug, Default)]
pub struct HttpServer {
    response_buffer: String<RESPONSE_BUFFER_SIZE>,
}

impl HttpHandler for HttpServer {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
        let start = Instant::now();
        let response = match (request.method, request.path) {
            (HttpMethod::GET, "/healthz") => HttpResponse {
                status_code: StatusCode::Ok,
                headers: Vec::new(),
                body: ResponseBody::Text("{\"status\":\"ok\"}"),
            },
            (HttpMethod::POST, "/pausez") => {
                Timer::after(Duration::from_millis(1000)).await;
                HttpResponse {
                    status_code: StatusCode::Ok,
                    headers: Vec::new(),
                    body: ResponseBody::Text("OK"),
                }
            }
            (HttpMethod::GET, "/metrics") => {
                self.response_buffer.clear();
                METRICS
                    .write_http_body(&mut self.response_buffer)
                    .map_or_else(
                        |_| HttpResponse {
                            status_code: StatusCode::RequestEntityTooLarge,
                            headers: Vec::new(),
                            body: ResponseBody::Text("Response too large"),
                        },
                        |()| HttpResponse {
                            status_code: StatusCode::Ok,
                            headers: Vec::new(),
                            body: ResponseBody::Text(self.response_buffer.as_str()),
                        },
                    )
            }
            _ => HttpResponse {
                status_code: StatusCode::NotFound,
                headers: Vec::new(),
                body: ResponseBody::Text("Not Found"),
            },
        };

        match request.method {
            HttpMethod::GET => {
                METRICS.http_get_requests.inc(1);
            }
            HttpMethod::POST => {
                METRICS.http_post_requests.inc(1);
            }

            _ => {}
        }

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
pub async fn run_http_server(stack: Stack<'static>, handler: HttpServer) {
    let mut server = DefaultHttpServer::new(80); // Listen on port 80

    info!("[HTTP] Serving http");
    pin!(server.serve(stack, handler)).await;
}
