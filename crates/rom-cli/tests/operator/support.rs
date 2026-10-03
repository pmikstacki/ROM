use rom::operator::*;
use serde_json::{Value, json};
use tokio::{io::AsyncWriteExt, net::TcpStream};

pub fn handle() -> WorkHandle {
    WorkHandle::from_work_id("notice-1")
}
pub fn view() -> Value {
    json!({"protocol_version":OPERATOR_PROTOCOL_VERSION,"handle":handle(),"version":{"generation":"store-1","revision":4},"category":"Notification","definition":{"name":"notice","version":1},"state":"Pending","attempts":1,"due":10,"delivery":"Unknown","source":null,"target":null})
}
pub fn request(reconcile: bool) -> Value {
    json!({"handle":handle(),"expected":{"generation":"store-1","revision":4},"key":"original-key","retry_epoch":7,"operation":if reconcile { json!({"Reconcile":{"evidence_ref":"provider-reference"}}) } else { json!("Retry") }})
}
pub fn result(reconcile: bool) -> Value {
    let request = request(reconcile);
    json!({"protocol_version":OPERATOR_PROTOCOL_VERSION,"handle":request["handle"],"version":{"generation":"store-1","revision":5},"key":request["key"],"operation":request["operation"],"outcome":if reconcile { "Completed" } else { "Scheduled" },"replayed":false})
}
pub fn page(records: Vec<Value>) -> Value {
    json!({"protocol_version":OPERATOR_PROTOCOL_VERSION,"records":records,"cursor":null})
}
pub async fn reply(socket: &mut TcpStream, status: u16, body: &Value) {
    let body = body.to_string();
    socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
}
pub async fn header(socket: &TcpStream) -> String {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let mut buffer = [0; 16_384];
            let size = socket.peek(&mut buffer).await.unwrap();
            assert_ne!(size, 0);
            if let Some(end) = buffer[..size]
                .windows(4)
                .position(|bytes| bytes == b"\r\n\r\n")
            {
                return String::from_utf8(buffer[..end].to_vec()).unwrap();
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap()
}
