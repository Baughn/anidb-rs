extern crate anidb;
extern crate rand;

use self::rand::Rng;
use std::net::UdpSocket;
use std::str;

use anidb::Result;

pub struct MockServer {
    pub socket: UdpSocket,
    pub token: String,
}

impl MockServer {
    pub fn new(port: u16) -> Result<MockServer> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        Ok(MockServer {
            socket: socket,
            token: rand::thread_rng().gen_ascii_chars().take(5).collect(),
        })
    }

    pub fn update(&self) {
        let mut buf = [0; 2048];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    println!("amt: {}", amt);
                    println!("src: {}", src);
                    println!("{}", str::from_utf8(&buf).unwrap_or(""));
                    let message = format!("200 {} LOGIN ACCEPTED\n", self.token);
                    println!("reply: {}", message);
                    self.socket.connect(src).unwrap();
                    self.socket.send(message.as_bytes()).unwrap();
                }
                Err(e) => {
                    println!("couldn't recieve a datagram: {}", e);
                }
            }
        }
    }
}
