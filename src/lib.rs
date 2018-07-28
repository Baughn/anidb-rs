extern crate crypto;
extern crate rusqlite;

mod cache;
mod cutil;
pub mod ed2k;
mod errors;
pub mod md4;

pub use errors::{AnidbError, Result};
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::str;
use std::thread;
use std::time::{Duration, Instant};

use std::net::UdpSocket;

use cache::Cache;
use ed2k::Ed2kHash;

pub struct Anidb {
    pub socket: UdpSocket,
    pub address: SocketAddr,
    pub session: Session,

    /// These are used to enforce flood protection.
    /// Don't override, Anidb will ban you.
    pub last_send: Instant,
    pub ratelimit: Duration,

    /// API cache.
    pub cache: Cache,
}

#[derive(Debug)]
pub struct ServerReply {
    pub code: i32,
    pub data: String,
}

#[derive(Debug)]
pub struct File {
    pub fid: u32,
    pub aid: u32,
    pub eid: u32,
    pub gid: u32,
    /// "Canonical" filename, as per AniDB.
    pub filename: String,
    pub total_eps: u32,
    pub highest_ep: u32,
    pub year: String,
    pub typ: String,
    pub series_romaji: String,
    pub series_english: String,
    pub series_other: String,
    pub series_short: String,
    /// The episode number can be non-numeric, e.g. for specials.
    pub ep_number: String,
    pub ep_name: String,
    pub ep_romaji: String,
    pub group_name: String,
    pub group_short: String,
}

#[derive(Debug)]
pub enum Session {
    Disconnected,
    Pending { user: String, pwd: String },
    Connected(String),
}

impl Anidb {
    ///
    /// Creates a new instance of Anidb and makes a connection to the AniDB API server
    /// ```ignore
    /// // code unwraps for simplicy but the error codes should be handled by the errors
    /// let mut db = anidb::Anidb::new(("api.anidb.net", 9000)).unwrap();
    /// ```
    ///
    pub fn new<A: ToSocketAddrs>(addr: A, cache_dir: &PathBuf) -> Result<Anidb> {
        let socket = UdpSocket::bind(("0.0.0.0", 0))?;
        socket.connect(&addr)?;

        Ok(Anidb {
            socket: socket,
            address: addr.to_socket_addrs().unwrap().next().unwrap(),
            session: Session::Disconnected,
            last_send: Instant::now(),
            ratelimit: Duration::from_secs(4),
            cache: Cache::new(cache_dir).expect("Cache creation failed"),
        })
    }

    /// Login the user to AniDB. You need to supply a user/pass that you have
    /// registered at https://anidb.net/
    ///
    /// The login is not actually executed until needed.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // code unwraps for simplicy but the error codes should be handled by the errors
    /// let mut db = anidb::Anidb::new(("api.anidb.net", 9000)).unwrap();
    /// db.login("leeloo_dallas", "multipass").unwrap();
    /// ```
    ///
    pub fn login(&mut self, username: &str, password: &str) -> Result<()> {
        self.session = Session::Pending {
            user: username.to_owned(),
            pwd: password.to_owned(),
        };
        Ok(())
    }

    /// Explicitly log out, e.g. to login as a different user.
    pub fn logout(&mut self) -> Result<()> {
        // TODO: Non-lexical lifetimes will let us simplify this.
        let logout_cmd = match self.session {
            Session::Connected(ref session) => Self::format_logout_string(session),
            _ => "".to_owned(),
        };
        if logout_cmd != "" {
            let reply = self.send_wait_reply(&logout_cmd)?;
            println!("Reply from server {}", reply.data);
        }
        self.session = Session::Disconnected;
        Ok(())
    }

    /// Search for a file, by hash.
    pub fn file_from_hash(&mut self, hash: &Ed2kHash) -> Result<File> {
        let file_str = Self::format_file_hash_str(hash);
        let reply = self.call_cached(&file_str)?;
        match reply.code {
            322 => Err(AnidbError::Error("Found multiple files. Panic!".to_owned())),
            320 => Err(AnidbError::NoSuchFile),
            220 => {
                let data = reply.data.split('\n').nth(1).expect("FILE format error");
                let mut fields = data.split('|');
                // The list of what we asked for.
                // Currently that's statically determined by the query format.
                let fid = fields.next().expect("fid not found");
                let aid = fields.next().expect("aid not found");
                let eid = fields.next().expect("eid not found");
                let gid = fields.next().expect("gid not found");
                let filename = fields.next().expect("filename not found");
                let total_eps = fields.next().expect("total_eps not found");
                let highest_ep = fields.next().expect("highest_ep not found");
                let year = fields.next().expect("year not found");
                let typ = fields.next().expect("typ not found");
                let series_romaji = fields.next().expect("series_romaji not found");
                let series_english = fields.next().expect("series_english not found");
                let series_other = fields.next().expect("series_other not found");
                let series_short = fields.next().expect("series_short not found");
                let ep_number = fields.next().expect("ep_number not found");
                let ep_name = fields.next().expect("ep_name not found");
                let ep_romaji = fields.next().expect("ep_romaji not found");
                let group_name = fields.next().expect("group_name not found");
                let group_short = fields.next().expect("group_short not found");

                Ok(File {
                    fid: fid.parse().expect("fid"),
                    aid: aid.parse().expect("aid"),
                    eid: eid.parse().expect("eid"),
                    gid: gid.parse().expect("gid"),
                    filename: filename.to_owned(),
                    total_eps: total_eps.parse().expect("total_eps"),
                    highest_ep: highest_ep.parse().expect("highest"),
                    year: year.to_owned(),
                    typ: typ.to_owned(),
                    series_romaji: series_romaji.to_owned(),
                    series_english: series_english.to_owned(),
                    series_other: series_other.to_owned(),
                    series_short: series_short.to_owned(),
                    ep_number: ep_number.to_owned(),
                    ep_name: ep_name.to_owned(),
                    ep_romaji: ep_romaji.to_owned(),
                    group_name: group_name.to_owned(),
                    group_short: group_short.to_owned(),
                })
            }
            code => Err(AnidbError::Error(format!("Unexpected code {}", code))),
        }
    }

    fn assert_session(&mut self) -> Result<String> {
        // TODO: Non-lexical lifetimes will let us simplify this.
        let login_cmd = match self.session {
            Session::Disconnected => String::new(),
            Session::Connected(_) => String::new(),
            Session::Pending { ref user, ref pwd } => Self::format_login_string(user, pwd),
        };
        if login_cmd != "" {
            let reply = self.send_wait_reply(&login_cmd)?;
            println!("Reply from server {}", reply.data);
            let session = Self::validate_auth_command(&reply)?;
            self.session = Session::Connected(session);
        }
        match self.session {
            Session::Connected(ref session) => Ok(session.clone()),
            _ => unreachable!(),
        }
    }

    /// Validates that the auth command has a correct reply from the server
    fn validate_auth_command(reply: &ServerReply) -> Result<String> {
        if reply.code != 200 {
            return Err(AnidbError::ErrorCode(reply.code, reply.data.to_owned()));
        }

        let v: Vec<&str> = reply.data.split(' ').collect();

        if v.len() != 3 {
            return Err(AnidbError::Error(format!(
                "Invalid AUTH reply: {} expceted 3 args",
                reply.data
            )));
        }

        if v[1] != "LOGIN" || v[2] != "ACCEPTED\n" {
            return Err(AnidbError::Error(format!(
                "Invalid AUTH reply: {} LOGIN ACCEPTED\\n expected",
                reply.data
            )));
        }

        Ok(v[0].to_owned())
    }

    /// Parse the reply from the server which is expected to be in xxx - format. If that is not the
    /// case this function will return an error that the reply couldn't be parsed.
    fn parse_reply(reply: &[u8], len: usize) -> Result<ServerReply> {
        if len < 5 {
            return Err(AnidbError::StaticError("Reply less than 5 chars"));
        }
        let code_str = str::from_utf8(&reply[0..3])?;
        let code = code_str.parse::<i32>()?;
        Ok(ServerReply {
            code: code,
            data: String::from_utf8_lossy(&reply[4..len]).into_owned(),
        })
    }

    fn send_wait_reply(&mut self, message: &str) -> Result<ServerReply> {
        let now = Instant::now();
        let period = now.duration_since(self.last_send);
        if period < self.ratelimit {
            thread::sleep(self.ratelimit - period);
        }
        self.last_send = Instant::now();
        let mut result = [0; 2048];
        self.socket.send(message.as_bytes())?;
        let len = self.socket.recv(&mut result)?;
        Self::parse_reply(&result, len)
    }

    fn call_cached(&mut self, message: &str) -> Result<ServerReply> {
        let cached = self.cache.get(message);
        match cached {
            Err(AnidbError::SqliteError(rusqlite::Error::QueryReturnedNoRows)) => {
                self.call(message)
            }
            Err(err) => Err(err),
            Ok(result) => Ok(result),
        }
    }

    fn call(&mut self, message: &str) -> Result<ServerReply> {
        let s = self.assert_session()?;
        let mws = format!("{}&s={}", message, s);
        let reply = self.send_wait_reply(&mws)?;
        println!("Reply from server {:?}", reply);
        self.cache.put(message, &reply)?;
        Ok(reply)
    }

    fn format_logout_string(session_id: &str) -> String {
        format!("LOGOUT s={}", session_id)
    }

    fn format_login_string(username: &str, password: &str) -> String {
        format!(
            "AUTH user={}&pass={}&protover=3&client=anidbrs&clientver=1",
            username, password
        )
    }

    fn format_file_hash_str(hash: &Ed2kHash) -> String {
        format!(
            "FILE size={}&ed2k={}&fmask=7000000100&amask=F0B8E0C0",
            hash.size, hash.hex
        )
    }
}

#[cfg(test)]
mod test_parse {
    use super::*;

    #[test]
    fn test_parse_reply_ok() {
        let reply = b"500 LOGIN FAILED";
        let ret = Anidb::parse_reply(reply, reply.len()).unwrap();
        assert_eq!(ret.code, 500);
        assert_eq!(ret.data, "LOGIN FAILED");
    }

    #[test]
    fn test_parse_reply_fail_1() {
        let reply = b"a3i5LOGIN FAILED";
        assert_eq!(true, Anidb::parse_reply(reply, reply.len()).is_err());
    }

    #[test]
    fn test_parse_reply_fail_2() {
        let reply = b"34i5LOGIN FAILED";
        assert_eq!(true, Anidb::parse_reply(reply, reply.len()).is_err());
    }

    #[test]
    fn test_parse_reply_too_short() {
        let reply = b"3D";
        assert_eq!(true, Anidb::parse_reply(reply, reply.len()).is_err());
    }

    #[test]
    fn test_parse_reply_exact_length() {
        let reply = b"777 O";
        let ret = Anidb::parse_reply(reply, reply.len()).unwrap();
        assert_eq!(ret.code, 777);
        assert_eq!(ret.data, "O");
    }

    fn test_parse_file() {
        let reply = b"220 FILE\n1879191|12235|183230|10435|Little Witch Academia (2017) - 01 - A New Beginning - [Asenshi](6a9d1e5c).mkv|25|25|2017-2017|TV Series|Little Witch Academia (2017)||???????????? (2017)'?? ?? ????? (2017)|lwatv|01|A New Beginning|Arata na Hajimari|AnimeSenshi Subs|Asenshi|1498599583";
    }
}

#[cfg(test)]
mod test_format {
    use super::*;

    #[test]
    fn test_format_login_string() {
        let login_string = Anidb::format_login_string("leeloo_dallas", "multipass");
        assert_eq!(
            login_string,
            "AUTH user=leeloo_dallas&pass=multipass&protover=3&client=anidbrs&clientver=1"
        );
    }

    #[test]
    fn test_format_logout_string() {
        let logout_str = Anidb::format_logout_string("abcd1234");
        assert_eq!(logout_str, "LOGOUT s=abcd1234");
    }
}
