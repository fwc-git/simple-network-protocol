#![deny(unused)]
#![deny(warnings)]
use std::{
    io::{BufRead, Read, Write},
    net::{self, TcpStream},
    thread,
};

use sha1::{Digest, Sha1};

const HTTP_RESPONSE: &str =
    "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n";

const HOST: &str = "127.0.0.1";

const PORT: u16 = 9090;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = net::TcpListener::bind("127.0.0.1:9090")?;
    loop {
        let (mut conn, addr) = match listener.accept() {
            Ok((conn, addr)) => (conn, addr),
            Err(e) => {
                println!("accept failed: {e}");
                break;
            }
        };
        println!("accept addr is: {}", addr);
        handle_shake(&mut conn);
        thread::spawn(move || {
            handle_stream(&mut conn);
        });
    }

    Ok(())
}

fn handle_shake(conn: &mut TcpStream) -> bool {
    let mut buf = [0u8; 1024];
    let _raw_data = conn.read(&mut buf);
    for header in buf.lines().skip(1) {
        let header = header.unwrap_or(String::from(""));
        println!("{header}");
        let mut headers = header.split(": ").into_iter();
        let webscoket_key = headers.find(|header| *header == "Sec-WebSocket-Key");
        match webscoket_key {
            Some(_) => {
                match send_shake_data(conn, headers.next().unwrap_or_default()) {
                    Ok(_) => {
                        println!("send data size is:");
                    }
                    Err(e) => {
                        println!("{e}");
                    }
                };
                return true;
            }
            None => {
                println!("{header}");
            }
        }
    }
    return false;
}

fn send_shake_data(conn: &mut TcpStream, webscoket_key: &str) -> Result<(), std::io::Error> {
    let mut hasher = Sha1::new();
    hasher.update(format!("{webscoket_key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes());
    let shake_data = base64::encode(hasher.finalize());
    println!("webscoket_key: {webscoket_key}");
    println!("Sec-WebSocket-Accept: {shake_data}");
    conn.write_all(
        format!(
            "{HTTP_RESPONSE}Sec-WebSocket-Accept: {shake_data}\r\nWebSocket-Location: ws://{HOST}:{PORT}\r\n\r\n"
        )
        .as_bytes(),
    )
}

fn handle_stream(conn: &mut TcpStream) {
    let mut buf = [0u8; 1024];
    while let Ok(recv_size) = conn.read(&mut buf) {
        println!("receive message size is: {recv_size}");
        if recv_size > 0 {
            unsafe {
                let mut payload = parse_payload(buf);
                // println!(
                //     "receive data is:{}",
                //     String::from_utf8(payload).unwrap_or_default()
                // );
                match conn.write(&pack_data(&mut payload)) {
                    Ok(send_size) => {
                        println!("send data size is: {send_size}");
                    }
                    Err(e) => {
                        println!("send message failed: {e}");
                    }
                };
            }
        }
    }
    // match conn.read(&mut buf) {
    //     Ok(recv_size) => {
    //         println!("receive message size is: {recv_size}");
    //         if recv_size > 0 {
    //             unsafe {
    //                 let mut payload = parse_payload(buf);
    //                 // println!(
    //                 //     "receive data is:{}",
    //                 //     String::from_utf8(payload).unwrap_or_default()
    //                 // );
    //                 match conn.write(&pack_data(&mut payload)) {
    //                     Ok(send_size) => {
    //                         println!("send data size is: {send_size}");
    //                     }
    //                     Err(e) => {
    //                         println!("send message failed: {e}");
    //                     }
    //                 };
    //             }
    //         }
    //     }
    //     Err(e) => {
    //         println!("receive message error: {e}");
    //     }
    // };
}

unsafe fn parse_payload(buf: [u8; 1024]) -> Vec<u8> {
    let fin = buf.get_unchecked(0) >> 7;
    let opcode = buf.get_unchecked(0) & 0b1111;
    let mask_flag = buf.get_unchecked(1) >> 7;
    let data_length = buf.get_unchecked(1) & 0b1111_111;
    let masks;
    let raw_data;
    println!("fin: {fin}, opcode: {opcode}, mask_flag: {mask_flag}, data_length: {data_length}");
    match data_length {
        126 => {
            masks = &buf[4..8];
            raw_data = &buf[8..];
        }
        127 => {
            masks = &buf[10..14];
            raw_data = &buf[14..];
        }
        _ => {
            masks = &buf[2..6];
            raw_data = &buf[6..];
        }
    }

    let mut index = 0;
    let mut data = Vec::with_capacity(data_length as usize);
    for b in raw_data.iter().take(data_length as usize) {
        data.push(b ^ masks.get_unchecked(index % 4));
        index += 1;
    }
    println!("{data:?}");
    return data;
}

fn pack_data(data: &mut Vec<u8>) -> Vec<u8> {
    let fin_and_opcode: u8 = 0b10000001;
    let mask_and_length = data.len();
    let mut packed_data = vec![fin_and_opcode];
    if mask_and_length < 126 {
        packed_data.push(mask_and_length as u8);
        packed_data.append(data);
        return packed_data;
    }
    return vec![fin_and_opcode, 0];
}

#[test]
fn hash_data() {
    let webscoket_key = "rmYNmfmNpxiXlRrNXQsuhw==";
    let mut hasher = Sha1::new();
    hasher.update(format!("{webscoket_key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes());

    let shake_data = base64::encode(hasher.finalize());
    assert_eq!(&shake_data, "GFaZHcMXnYYatvVSdKl/oQUUgQM=");
}
