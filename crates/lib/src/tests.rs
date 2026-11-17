use super::Client;
use radicula_oidc::Secret;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

fn reply(body: &str) -> (Client, JoinHandle<String>) {
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", socket.local_addr().unwrap());
    let body = body.to_string();
    let job = std::thread::spawn(move || {
        let (mut stream, _) = socket.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut received = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let size = stream.read(&mut buffer).unwrap();
            received.extend_from_slice(&buffer[..size]);
            if complete(&received) {
                break;
            }
            assert!(size > 0 && received.len() < 16384);
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(received).unwrap()
    });
    (
        Client {
            base,
            token: Secret::new("held-login-only".into()),
        },
        job,
    )
}

#[test]
fn identity() {
    let (client, wire) = reply(r#"{"sub":"subject-7","login":"ada","name":"Ada"}"#);
    assert_eq!(client.identity().unwrap().sub, "subject-7");
    let request = wire.join().unwrap();
    assert!(request.starts_with("GET /whoami HTTP/1.1"));
    assert!(
        request
            .to_lowercase()
            .contains("authorization: token held-login-only")
    );
}

#[test]
fn exchange() {
    let (client, wire) = reply(
        r#"{"access_token":"short-lived","issued_token_type":"urn:ietf:params:oauth:token-type:access_token","token_type":"Bearer","expires_in":600}"#,
    );
    let token = client.exchange("https://cloud.example/audience").unwrap();
    assert_eq!(token.token.expose(), "short-lived");
    assert!(!format!("{token:?}").contains("short-lived"));
    let request = wire.join().unwrap();
    assert!(request.starts_with("POST /token HTTP/1.1"));
    assert!(request.contains("subject_token=held-login-only"));
    assert!(request.contains("resource=https%3A%2F%2Fcloud.example%2Faudience"));
    assert!(!request.to_lowercase().contains("authorization:"));
}

#[test]
fn malformed() {
    let (client, wire) = reply(
        r#"{"access_token":"secret-reply","issued_token_type":"wrong","token_type":"Bearer","expires_in":600}"#,
    );
    let error = client.exchange("cloud").unwrap_err();
    assert!(!error.contains("secret-reply"));
    wire.join().unwrap();
}

#[test]
fn subject() {
    let (client, wire) = reply(r#"{"sub":"","login":"ada","name":"Ada"}"#);
    assert!(client.identity().is_err());
    wire.join().unwrap();
}

#[test]
fn prefix() {
    assert_eq!(super::config::Profile::prefix(), "ENSIGN");
}

fn length(header: &str) -> usize {
    header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0)
}

fn complete(received: &[u8]) -> bool {
    let text = String::from_utf8_lossy(received);
    let Some((header, body)) = text.split_once("\r\n\r\n") else {
        return false;
    };
    if header
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        return body.ends_with("0\r\n\r\n");
    }
    body.len() >= length(header)
}

#[test]
fn discovery() {
    let (client, wire) = reply(
        r#"{"issuer":"https://identity.example/api","token_endpoint":"https://identity.example/api/token","jwks_uri":"https://identity.example/api/.well-known/jwks.json"}"#,
    );
    assert_eq!(
        client.discovery().unwrap().issuer,
        "https://identity.example/api"
    );
    let request = wire.join().unwrap();
    assert!(request.starts_with("GET /.well-known/openid-configuration HTTP/1.1"));
    assert!(!request.contains("held-login-only"));
}
