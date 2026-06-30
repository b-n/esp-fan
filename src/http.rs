use core::fmt;
use core::pin::pin;
use defmt::info;
use embassy_net::Stack;
use embassy_time::{Duration, Instant, Timer};
use heapless::Vec;
use nanofish::{
    DefaultHttpServer, HttpHandler, HttpMethod, HttpRequest, HttpResponse, ResponseBody, StatusCode,
};

use crate::prom::METRICS;

const RESPONSE_BUFFER_SIZE: usize = 65535;

#[derive(Debug)]
/// A response buffer that can be used for writing and returning responses.
///
/// Why? Because why use alloc when you could make your life harder
struct HttpResponseBuffer {
    buf: [u8; RESPONSE_BUFFER_SIZE],
    len: usize,
}

#[allow(clippy::large_stack_arrays)]
impl Default for HttpResponseBuffer {
    fn default() -> Self {
        Self {
            buf: [0; RESPONSE_BUFFER_SIZE],
            len: 0,
        }
    }
}

impl HttpResponseBuffer {
    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn as_str(&self) -> &str {
        // Safety: We only write valid UTF8
        unsafe { str::from_utf8_unchecked(&self.buf[..self.len]) }
    }
}

impl fmt::Write for HttpResponseBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let mut to_write = bytes.len();

        if self.len + to_write > self.buf.len() {
            to_write = self.buf.len() - self.len;
        }

        self.buf[self.len..self.len + to_write].copy_from_slice(bytes);
        self.len += to_write;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct HttpServer {
    response_buffer: HttpResponseBuffer,
}

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
            "/metrics" => {
                self.response_buffer.clear();
                METRICS.write_http_body(&mut self.response_buffer).unwrap();
                let body = ResponseBody::Text(self.response_buffer.as_str());

                HttpResponse {
                    status_code: StatusCode::Ok,
                    headers: Vec::new(),
                    body,
                }
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
