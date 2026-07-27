use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::{self, Read, Write};

/// Read one Chrome/Firefox native-messaging framed message (u32 LE length + JSON).
pub fn read_message<T: DeserializeOwned, R: Read>(reader: &mut R) -> io::Result<Option<T>> {
    let mut len_buf = [0_u8; 4];
    match reader.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 {
        return Ok(None);
    }
    if len > 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("native messaging payload too large: {len} bytes"),
        ));
    }
    let mut body = vec![0_u8; len];
    reader.read_exact(&mut body)?;
    let value = serde_json::from_slice(&body).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid native messaging JSON: {error}"),
        )
    })?;
    Ok(Some(value))
}

/// Write one native-messaging framed message.
pub fn write_message<T: Serialize, W: Write>(writer: &mut W, value: &T) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("serialize native messaging JSON: {error}"),
        )
    })?;
    if body.len() > u32::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "native messaging payload exceeds u32 length",
        ));
    }
    let len = (body.len() as u32).to_le_bytes();
    writer.write_all(&len)?;
    writer.write_all(&body)?;
    writer.flush()?;
    Ok(())
}

/// Read one JSON value per line (dev/test mode).
#[allow(dead_code)]
pub fn read_jsonl_message<T: DeserializeOwned, R: Read>(reader: &mut R) -> io::Result<Option<T>> {
    let mut line = String::new();
    let mut byte = [0_u8; 1];
    loop {
        match reader.read(&mut byte)? {
            0 => {
                if line.is_empty() {
                    return Ok(None);
                }
                break;
            }
            _ => {
                let ch = byte[0] as char;
                if ch == '\n' {
                    break;
                }
                if ch != '\r' {
                    line.push(ch);
                }
            }
        }
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let value = serde_json::from_str(trimmed).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSONL message: {error}"),
        )
    })?;
    Ok(Some(value))
}

pub fn write_jsonl_message<T: Serialize, W: Write>(writer: &mut W, value: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn framed_roundtrip() {
        let msg = json!({"id": 1, "method": "ping"});
        let mut buf = Vec::new();
        write_message(&mut buf, &msg).expect("write");
        let mut cursor = Cursor::new(buf);
        let decoded: serde_json::Value = read_message(&mut cursor).expect("read").expect("some");
        assert_eq!(decoded["method"], "ping");
    }
}
