use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU16, Ordering},
};
use std::thread::JoinHandle;

pub(super) struct Server {
    pub(super) url: String,
    status: Arc<AtomicU16>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    pub(super) fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/api", listener.local_addr().unwrap());
        let status = Arc::new(AtomicU16::new(200));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let status = status.clone();
            let stop = stop.clone();
            std::thread::spawn(move || serve(listener, status, stop))
        };
        Self {
            url,
            status,
            stop,
            worker: Some(worker),
        }
    }

    pub(super) fn status(&self, status: u16) {
        self.status.store(status, Ordering::SeqCst);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn serve(listener: TcpListener, status: Arc<AtomicU16>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut socket, _)) => reply(&mut socket, status.load(Ordering::SeqCst)),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(5))
            }
            Err(error) => panic!("listener refused: {error}"),
        }
    }
}

fn request(socket: &mut std::net::TcpStream) -> String {
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let count = socket.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if complete(&bytes) {
            break;
        }
    }
    String::from_utf8(bytes).unwrap()
}

fn complete(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return false;
    };
    if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        return body.ends_with("0\r\n\r\n");
    }
    let length = head
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);
    body.len() >= length
}

fn reply(socket: &mut std::net::TcpStream, wanted: u16) {
    let request = request(socket);
    let posted = request.starts_with("POST /api/bearer ");
    if posted {
        assert!(request.contains("private-password"));
        assert!(request.contains("private-login"));
    } else {
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: token private-token")
        );
    }
    let code = if posted && wanted == 200 { 201 } else { wanted };
    let body = if posted {
        "{\"token\":\"private-token\"}"
    } else if request.starts_with("GET /api/Actor ") {
        "[{\"login\":\"private-login\",\"name\":\"private-name\",\"kind\":\"human\"}]"
    } else {
        "{\"id\":7,\"login\":\"private-login\",\"name\":\"private-name\"}"
    };
    let response = format!(
        "HTTP/1.1 {code} OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(response.as_bytes()).unwrap();
}
