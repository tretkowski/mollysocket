use async_trait::async_trait;
use futures_channel::mpsc;
use tokio_tungstenite::tungstenite;
use websocket_connection::{
    websocket_message::{
        webSocketMessage::Type, WebSocketMessage, WebSocketRequestMessage, WebSocketResponseMessage,
    },
    WebSocketConnection,
};

pub mod tls;
pub mod websocket_connection;

const SERVER_DELIVERED_TIMESTAMP_HEADER: &str = "X-Signal-Timestamp";

pub struct SignalWebSocket {
    connect_addr: url::Url,
    push_endpoint: url::Url,
    tx: Option<mpsc::UnboundedSender<tungstenite::Message>>,
}

#[async_trait(?Send)]
impl WebSocketConnection for SignalWebSocket {
    fn get_url(&self) -> &url::Url {
        &self.connect_addr
    }

    fn get_tx(&self) -> &Option<mpsc::UnboundedSender<tungstenite::Message>> {
        &self.tx
    }

    fn set_tx(&mut self, tx: Option<mpsc::UnboundedSender<tungstenite::Message>>) {
        self.tx = tx
    }

    async fn on_message(&self, message: WebSocketMessage) {
        match message.r#type {
            Some(type_int) => match Type::from_i32(type_int) {
                Some(Type::RESPONSE) => (),
                Some(Type::REQUEST) => self.on_request(message.request).await,
                _ => (),
            },
            None => (),
        };
    }
}

impl SignalWebSocket {
    pub fn new(connect_addr: String, push_endpoint: String) -> Self {
        let connect_addr = url::Url::parse(&connect_addr).expect("Cannot parse websocket url");
        let push_endpoint = url::Url::parse(&push_endpoint).expect("Cannot parse endpoint url");
        Self {
            connect_addr,
            push_endpoint,
            tx: None,
        }
    }
    /**
     * That's when we must send a notification
     */
    async fn on_request(&self, request: Option<WebSocketRequestMessage>) {
        if let Some(request) = request {
            if self.read_or_empty(request) {
                self.notify().await;
            }
        }
    }

    fn read_or_empty(&self, request: WebSocketRequestMessage) -> bool {
        dbg!(&request.path);
        let response = self.create_websocket_response(&request);
        dbg!(&response);
        if self.is_signal_service_envelope(&request) {
            let timestamp: u64 = match self.find_header(&request) {
                Some(timestamp) => timestamp.parse().unwrap(),
                None => 0,
            };
            self.send_response(response);
            return true;
        }
        false
    }

    fn is_signal_service_envelope(
        &self,
        WebSocketRequestMessage {
            verb,
            path,
            body: _,
            headers: _,
            id: _,
        }: &WebSocketRequestMessage,
    ) -> bool {
        if let Some(verb) = verb {
            if let Some(path) = path {
                return verb.eq("PUT") && path.eq("/api/v1/message");
            }
        }
        false
    }

    fn is_socket_empty_request(
        &self,
        WebSocketRequestMessage {
            verb,
            path,
            body: _,
            headers: _,
            id: _,
        }: &WebSocketRequestMessage,
    ) -> bool {
        if let Some(verb) = verb {
            if let Some(path) = path {
                return verb.eq("PUT") && path.eq("/api/v1/queue/empty");
            }
        }
        false
    }

    fn create_websocket_response(
        &self,
        request: &WebSocketRequestMessage,
    ) -> WebSocketResponseMessage {
        if self.is_signal_service_envelope(request) {
            return WebSocketResponseMessage {
                id: request.id,
                status: Some(200),
                message: Some(String::from("OK")),
                headers: Vec::new(),
                body: None,
            };
        }
        WebSocketResponseMessage {
            id: request.id,
            status: Some(400),
            message: Some(String::from("Unknown")),
            headers: Vec::new(),
            body: None,
        }
    }

    fn find_header(&self, message: &WebSocketRequestMessage) -> Option<String> {
        if message.headers.len() == 0 {
            return None;
        }
        let mut header_iter = message.headers.iter().filter_map(|header| {
            if header
                .to_lowercase()
                .starts_with(SERVER_DELIVERED_TIMESTAMP_HEADER)
            {
                let mut split = header.split(":");
                if let Some(header_name) = split.next() {
                    if let Some(header_value) = split.next() {
                        if header_name
                            .trim()
                            .eq_ignore_ascii_case(SERVER_DELIVERED_TIMESTAMP_HEADER)
                        {
                            return Some(String::from(header_value.to_lowercase().trim()));
                        }
                    }
                }
            }
            None
        });
        header_iter.next()
    }

    async fn notify(&self) {
        println!("Notifying");
        let url = self.push_endpoint.clone();
        let _ = reqwest::Client::new()
            .post(url)
            .header("Content-Type", "application/json")
            .body("{\"type\":\"request\"}")
            .send()
            .await;
    }
}