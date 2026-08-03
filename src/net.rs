use std::io::{Read, Write};
use std::net::TcpStream;

/// Largest frame either side will send or accept. Real messages are well under
/// 64 KiB; anything bigger is a corrupt or hostile peer.
const MAX_MSG_SIZE: usize = 1024 * 1024;

pub fn send_msg<T: serde::Serialize>(stream: &mut TcpStream, msg: &T) -> std::io::Result<()> {
    let bytes = bincode::serialize(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if bytes.len() > MAX_MSG_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("message length {} exceeds limit {}", bytes.len(), MAX_MSG_SIZE),
        ));
    }
    let len = bytes.len() as u32;
    stream.write_all(&len.to_le_bytes())?;
    stream.write_all(&bytes)?;
    Ok(())
}

pub fn recv_msg<T: serde::de::DeserializeOwned>(stream: &mut TcpStream) -> std::io::Result<T> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_MSG_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("message length {} exceeds limit {}", len, MAX_MSG_SIZE),
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    bincode::deserialize(&buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn socket_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    #[test]
    fn recv_msg_rejects_oversized_frame_without_allocating() {
        let (mut client, mut server) = socket_pair();
        // Claim a 4 GiB - 1 payload; only the header is ever sent.
        client.write_all(&u32::MAX.to_le_bytes()).unwrap();
        let err = recv_msg::<String>(&mut server).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn send_recv_round_trip() {
        let (mut client, mut server) = socket_pair();
        send_msg(&mut client, &String::from("hello")).unwrap();
        let msg: String = recv_msg(&mut server).unwrap();
        assert_eq!(msg, "hello");
    }

    #[test]
    fn send_msg_rejects_oversized_payload() {
        let (mut client, _server) = socket_pair();
        let big = vec![0u8; MAX_MSG_SIZE + 1];
        let err = send_msg(&mut client, &big).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
