//! Bounded JSON-RPC framing shared by HTTP SSE and stdio transports.
use std::io::{self, BufRead};
pub const MESSAGE_LIMIT: usize = 4 * 1024 * 1024;
pub const CATALOG_LIMIT: usize = 16 * 1024 * 1024;

pub fn read_line(
    reader: &mut impl BufRead,
    output: &mut String,
    limit: usize,
) -> io::Result<usize> {
    output.clear();
    let mut bytes = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }
        let count = available
            .iter()
            .position(|b| *b == b'\n')
            .map_or(available.len(), |i| i + 1);
        if bytes.len().saturating_add(count) > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MCP frame exceeds size limit",
            ));
        }
        let finished = available[count - 1] == b'\n';
        bytes.extend_from_slice(&available[..count]);
        reader.consume(count);
        if finished {
            break;
        }
    }
    let count = bytes.len();
    *output =
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unterminated_oversize_before_consuming_it() {
        let mut reader = io::BufReader::with_capacity(8, io::Cursor::new(vec![b'a'; 128]));
        let mut line = String::new();
        assert!(read_line(&mut reader, &mut line, 16).is_err());
        assert!(line.is_empty());
    }
    #[test]
    fn preserves_utf8_and_frame_boundaries() {
        let mut reader = io::BufReader::with_capacity(2, io::Cursor::new("hé\nnext\n".as_bytes()));
        let mut line = String::new();
        assert_eq!(read_line(&mut reader, &mut line, 8).unwrap(), 4);
        assert_eq!(line, "hé\n");
        assert_eq!(read_line(&mut reader, &mut line, 8).unwrap(), 5);
        assert_eq!(line, "next\n");
    }
}
