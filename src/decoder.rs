use crate::MULTIPLIER;
use crate::*;

#[allow(dead_code)]
pub fn decode(mut buffer: Vec<u8>) -> Option<Packet> {
    if let Some((header, header_size)) = read_header(&buffer) {
        dbg!(header_size);
        buffer = buffer.split_off(header_size); //removing header bytes, possible ALLOC
        if header.len() == 0 {
            let p = match header.packet() {
                PacketType::PingReq => Packet::PingReq,
                PacketType::PingResp => Packet::PingResp,
                PacketType::Disconnect => Packet::Disconnect,
                _ => {
                    println!("Phantom Packet. Error ");
                    Packet::None
                }
            };
            Some(p)
        } else if buffer.len() >= header.len() {
            let remaining = buffer.split_off(header.len());
            let p = read_packet(header.packet(), buffer);
            buffer = remaining;
            Some(p)
        } else {
            None
        }
    } else {
        None
    }
}

fn read_packet(t: PacketType, buffer: Vec<u8>) -> Packet {
    match t {
        PacketType::Connect => Packet::None,
        PacketType::Connack => Packet::None,
        PacketType::Publish => Packet::None,
        PacketType::Puback => Packet::None,
        PacketType::Pubrec => Packet::None,
        PacketType::Pubrel => Packet::None,
        PacketType::PubComp => Packet::None,
        PacketType::Subscribe => Packet::None,
        PacketType::SubAck => Packet::None,
        PacketType::UnSubscribe => Packet::None,
        PacketType::UnSubAck => Packet::None,
        _ => {
            println!("Phantom Packet. Error ");
            Packet::None
        }
    }
}
/* This will read the header of the stream */
fn read_header(buffer: &Vec<u8>) -> Option<(Header, usize)> {
    if buffer.len() > 1 {
        let header_u8 = buffer.get(0).unwrap();
        if let Some((length, size)) = read_length(buffer, 1) {
            let header = Header::new(*header_u8, length).unwrap();
            Some((header, size + 1))
        } else {
            None
        }
    } else {
        None
    }
}

fn read_length(buffer: &Vec<u8>, mut pos: usize) -> Option<(usize, usize)> {
    let mut mult: usize = 1;
    let mut len: usize = 0;
    let mut done = false;

    while !done {
        let byte = (*buffer.get(pos).unwrap()) as usize;
        len += (byte & 0x7F) * mult;
        mult *= 0x80;
        if mult > MULTIPLIER {
            return None;
        }
        if (byte & 0x80) == 0 {
            done = true;
        } else {
            pos += 1;
        }
    }
    Some((len as usize, pos))
}