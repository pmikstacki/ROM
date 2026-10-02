use crate::Failure;
pub const FRAME_LIMIT: usize = 2 * 1024 * 1024;
pub struct Frame {
    pub event: String,
    pub data: String,
}
#[derive(Default)]
pub struct Parser {
    line: Vec<u8>,
    event: String,
    data: String,
    has_data: bool,
    size: usize,
    cr: bool,
}
impl Parser {
    pub fn byte(&mut self, byte: u8) -> Result<Option<Frame>, Failure> {
        self.size += 1;
        if self.size > FRAME_LIMIT {
            return Err(Failure::transport("SSE frame exceeds 2 MiB"));
        }
        if self.cr {
            self.cr = false;
            if byte == b'\n' {
                return Ok(None);
            }
        }
        if matches!(byte, b'\r' | b'\n') {
            self.cr = byte == b'\r';
            let line = std::str::from_utf8(&self.line)
                .map_err(|_| Failure::transport("invalid SSE encoding"))?;
            if line.is_empty() {
                let frame = if self.has_data {
                    Some(Frame {
                        event: std::mem::take(&mut self.event),
                        data: std::mem::take(&mut self.data),
                    })
                } else {
                    None
                };
                self.event.clear();
                self.data.clear();
                self.has_data = false;
                self.size = 0;
                return Ok(frame);
            }
            let (field, mut value) = line.split_once(':').unwrap_or((line, ""));
            if let Some(v) = value.strip_prefix(' ') {
                value = v;
            }
            match field {
                "event" => self.event = value.into(),
                "data" => {
                    if self.has_data {
                        self.data.push('\n');
                    }
                    self.data.push_str(value);
                    self.has_data = true;
                }
                "" | "id" | "retry" => {}
                _ => {}
            }
            self.line.clear();
        } else {
            self.line.push(byte);
        }
        Ok(None)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crlf_bytes_all_count_against_frame_budget() {
        let mut parser = Parser::default();
        let mut rejected = false;
        for byte in b":\r\n".iter().copied().cycle().take(FRAME_LIMIT + 1) {
            if parser.byte(byte).is_err() {
                rejected = true;
                break;
            }
        }
        assert!(rejected, "CRLF wire bytes must not evade the frame cap");
    }
}
