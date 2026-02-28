#[derive(Debug, Default)]
pub(crate) struct SseEventParser {
    buffer: Vec<u8>,
    data_lines: Vec<String>,
}

impl SseEventParser {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);

        let mut events = Vec::new();
        let mut consumed = 0usize;
        while let Some(rel_pos) = self.buffer[consumed..]
            .iter()
            .position(|byte| *byte == b'\n')
        {
            let line_end = consumed + rel_pos;
            let line = self.buffer[consumed..line_end].to_vec();
            self.process_line(&line, &mut events);
            consumed = line_end + 1;
        }
        if consumed > 0 {
            self.buffer.drain(..consumed);
        }
        events
    }

    pub(crate) fn finish(&mut self) -> Vec<String> {
        let mut events = Vec::new();

        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            self.process_line(&line, &mut events);
        }
        self.flush_event(&mut events);

        events
    }

    fn process_line(&mut self, line_bytes: &[u8], events: &mut Vec<String>) {
        let line_bytes = line_bytes.strip_suffix(b"\r").unwrap_or(line_bytes);

        if line_bytes.is_empty() {
            self.flush_event(events);
            return;
        }

        if line_bytes.starts_with(b":") {
            return;
        }

        if let Some(raw_data) = line_bytes.strip_prefix(b"data:") {
            let data = raw_data.strip_prefix(b" ").unwrap_or(raw_data);
            self.data_lines
                .push(String::from_utf8_lossy(data).into_owned());
        }
    }

    fn flush_event(&mut self, events: &mut Vec<String>) {
        if self.data_lines.is_empty() {
            return;
        }
        events.push(self.data_lines.join("\n"));
        self.data_lines.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::SseEventParser;

    #[test]
    fn test_sse_parser_parses_single_event() {
        let mut parser = SseEventParser::new();
        let events = parser.push(
            br#"data: {"ok":true}

"#,
        );
        assert_eq!(events, vec![r#"{"ok":true}"#.to_string()]);
    }

    #[test]
    fn test_sse_parser_parses_multiline_data() {
        let mut parser = SseEventParser::new();
        let events = parser.push(b"event: message\ndata: first\ndata: second\n\n");
        assert_eq!(events, vec!["first\nsecond".to_string()]);
    }

    #[test]
    fn test_sse_parser_handles_utf8_split_across_chunks() {
        let mut parser = SseEventParser::new();
        let full = "data: 😀\n\n".as_bytes();
        let split_index = "data: ".len() + 1;

        let first = parser.push(&full[..split_index]);
        assert!(first.is_empty());

        let second = parser.push(&full[split_index..]);
        assert_eq!(second, vec!["😀".to_string()]);
    }

    #[test]
    fn test_sse_parser_handles_crlf() {
        let mut parser = SseEventParser::new();
        let events = parser.push(b"data: hello\r\n\r\n");
        assert_eq!(events, vec!["hello".to_string()]);
    }

    #[test]
    fn test_sse_parser_finish_flushes_last_event_without_terminator() {
        let mut parser = SseEventParser::new();
        assert!(parser.push(b"data: tail").is_empty());
        assert_eq!(parser.finish(), vec!["tail".to_string()]);
    }
}
