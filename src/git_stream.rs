use std::io::{self, BufRead, ErrorKind, Read};
use std::process::Child;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::Duration;

/// Upper bound for one unified-diff body line. Real Rust sources stay far
/// below this; the bound exists so a hostile or corrupt stream cannot force an
/// unbounded allocation before Cultist fails closed.
pub(crate) const MAX_DIFF_LINE_BYTES: usize = 16 * 1024 * 1024;

/// Upper bound for one `git log` metadata or path line (SHAs, dates, subjects,
/// changed paths). Subjects and paths beyond this size are treated as malformed.
pub(crate) const MAX_LOG_LINE_BYTES: usize = 64 * 1024;

/// Upper bound for one `--name-only` path line.
pub(crate) const MAX_PATH_LINE_BYTES: usize = 8 * 1024;

/// Reads one line into `line`, including its `\n` delimiter, mirroring
/// [`BufRead::read_line`] semantics while refusing to buffer more than
/// `max_bytes`. Returns the number of bytes read; `Ok(0)` marks end of stream.
/// A longer line fails closed with [`ErrorKind::InvalidData`] instead of
/// growing the allocation.
pub(crate) fn read_line_bounded<R: BufRead>(
    reader: &mut R,
    line: &mut Vec<u8>,
    max_bytes: usize,
) -> io::Result<usize> {
    line.clear();
    let mut total = 0_usize;

    loop {
        let available = match reader.fill_buf() {
            Ok(available) => available,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if available.is_empty() {
            return Ok(total);
        }

        let consumed = match available.iter().position(|byte| *byte == b'\n') {
            Some(newline) => newline + 1,
            None => available.len(),
        };

        if total + consumed > max_bytes {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("streamed line exceeded {max_bytes} bytes"),
            ));
        }
        line.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        total += consumed;

        if line.ends_with(b"\n") {
            return Ok(total);
        }
    }
}

/// Reads one bounded UTF-8 text line, trimming nothing. `Ok(0)` marks EOF.
pub(crate) fn read_text_line_bounded<R: BufRead>(
    reader: &mut R,
    text: &mut String,
    scratch: &mut Vec<u8>,
    max_bytes: usize,
) -> io::Result<usize> {
    let read = read_line_bounded(reader, scratch, max_bytes)?;
    if read == 0 {
        text.clear();
        return Ok(0);
    }
    let decoded = std::str::from_utf8(scratch)
        .map_err(|_| io::Error::new(ErrorKind::InvalidData, "streamed line is not valid UTF-8"))?;
    text.clear();
    text.push_str(decoded);
    Ok(read)
}

/// Handle to a child's stderr being drained on a worker thread so a chatty
/// child cannot deadlock on a full pipe while stdout is being parsed.
pub(crate) struct StderrDrain {
    receiver: Receiver<String>,
}

impl StderrDrain {
    /// Waits for the drain to finish and returns the captured text (lossily
    /// decoded). Call only once the child has been waited on normally; orphaned
    /// grandchildren can hold the pipe open, so the termination path uses
    /// [`terminate_child`] instead.
    pub(crate) fn finish(self) -> String {
        self.receiver.recv().unwrap_or_default()
    }
}

/// Spawns a background drainer for a child's stderr. The returned handle
/// yields the captured text once joined via [`StderrDrain::finish`].
pub(crate) fn drain_stderr<R: Read + Send + 'static>(stderr: R) -> StderrDrain {
    let (sender, receiver) = channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let text = match io::BufReader::new(stderr).read_to_end(&mut bytes) {
            Ok(_) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(_) => String::new(),
        };
        let _ = sender.send(text);
    });
    StderrDrain { receiver }
}

/// Kills and reaps a child whose streamed output could not be parsed, then
/// collects whatever stderr was drained within a short grace period.
///
/// The grace period matters because killed parents can leave orphaned
/// descendants holding the stderr write end; blocking on full EOF there would
/// stall error reporting on processes Cultist does not own.
pub(crate) fn terminate_child(mut child: Child, stderr_drain: StderrDrain) {
    let _ = child.kill();
    let _ = child.wait();
    // A short grace period only; orphans holding the pipe must not stall us.
    let _ = stderr_drain
        .receiver
        .recv_timeout(Duration::from_millis(250));
}

/// Outcome of [`c_style_unquote`].
pub(crate) enum UnquotedPath {
    /// The token was not C-quoted; it is used verbatim.
    Verbatim,
    /// The token was quoted and decoded successfully into raw path bytes.
    Decoded(Vec<u8>),
}

/// Decodes a Git C-style quoted path token (`"..."` with `\"`, `\\`,
/// `\a b f n r t v`, and three-digit octal escapes). Returns
/// [`UnquotedPath::Verbatim`] when the token does not start with a quote, and
/// an error when it is quoted but malformed.
pub(crate) fn c_style_unquote(token: &str) -> Result<UnquotedPath, String> {
    if !token.starts_with('"') {
        return Ok(UnquotedPath::Verbatim);
    }
    if token.len() < 2 || !token.ends_with('"') {
        return Err("quoted path is missing its closing quote".to_string());
    }

    let inner = &token[1..token.len() - 1];
    let mut bytes = Vec::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(char) = chars.next() {
        match char {
            '"' => return Err("quoted path contains a raw double quote".to_string()),
            '\\' => {
                let escape = chars
                    .next()
                    .ok_or_else(|| "quoted path ends with a dangling backslash".to_string())?;
                match escape {
                    '"' => bytes.push(b'"'),
                    '\\' => bytes.push(b'\\'),
                    'a' => bytes.push(0x07),
                    'b' => bytes.push(0x08),
                    'f' => bytes.push(0x0c),
                    'n' => bytes.push(b'\n'),
                    'r' => bytes.push(b'\r'),
                    't' => bytes.push(b'\t'),
                    'v' => bytes.push(0x0b),
                    '0'..='7' => {
                        let mut value = escape.to_digit(8).expect("octal digit");
                        for _ in 0..2 {
                            let digit = chars.next().ok_or_else(|| {
                                "quoted path octal escape is truncated".to_string()
                            })?;
                            value = value * 8
                                + digit
                                    .to_digit(8)
                                    .ok_or("quoted path octal escape has a non-octal digit")?;
                        }
                        if value > u8::MAX as u32 {
                            return Err("quoted path octal escape exceeds one byte".to_string());
                        }
                        bytes.push(value as u8);
                    }
                    other => {
                        return Err(format!("quoted path uses unsupported escape `\\{other}`"));
                    }
                }
            }
            other => {
                let mut buffer = [0_u8; 4];
                bytes.extend_from_slice(other.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }

    Ok(UnquotedPath::Decoded(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounded_lines(input: &[u8], max: usize) -> Vec<String> {
        let mut reader = io::Cursor::new(input);
        let mut text = String::new();
        let mut scratch = Vec::new();
        let mut lines = Vec::new();
        loop {
            let read = read_text_line_bounded(&mut reader, &mut text, &mut scratch, max).unwrap();
            if read == 0 {
                break;
            }
            lines.push(text.trim_end_matches(['\n', '\r']).to_string());
        }
        lines
    }

    #[test]
    fn reads_lines_with_and_without_trailing_newlines() {
        assert_eq!(
            bounded_lines(b"one\ntwo\nthree", 1024),
            vec!["one".to_string(), "two".to_string(), "three".to_string()]
        );
        assert!(bounded_lines(b"", 16).is_empty());
    }

    #[test]
    fn fragmented_fills_reassemble_exactly() {
        struct Trickle<R> {
            inner: R,
        }
        impl<R: Read> Read for Trickle<R> {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if buf.is_empty() {
                    return Ok(0);
                }
                buf[0] = 0;
                self.inner.read(&mut buf[..1])
            }
        }

        let mut reader = io::BufReader::new(Trickle {
            inner: io::Cursor::new(b"alpha\r\nbeta\n".to_vec()),
        });
        let mut text = String::new();
        let mut scratch = Vec::new();
        let mut lines = Vec::new();
        while read_text_line_bounded(&mut reader, &mut text, &mut scratch, 64).unwrap() != 0 {
            lines.push(text.trim_end_matches(['\n', '\r']).to_string());
        }
        assert_eq!(lines, vec!["alpha", "beta"]);
    }

    #[test]
    fn over_limit_lines_fail_closed_with_invalid_data() {
        let mut reader = io::Cursor::new(b"ok\nway-too-long\n".to_vec());
        let mut text = String::new();
        let mut scratch = Vec::new();
        assert_eq!(
            read_text_line_bounded(&mut reader, &mut text, &mut scratch, 3).unwrap(),
            3
        );
        assert_eq!(text.trim_end_matches(['\n']), "ok");
        assert_eq!(
            read_text_line_bounded(&mut reader, &mut text, &mut scratch, 3)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn non_utf8_lines_fail_closed() {
        let mut reader = io::Cursor::new(vec![b'a', 0xff, b'\n']);
        let mut text = String::new();
        let mut scratch = Vec::new();
        assert_eq!(
            read_text_line_bounded(&mut reader, &mut text, &mut scratch, 64)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_git_c_style_paths() {
        use UnquotedPath::*;
        assert!(matches!(c_style_unquote("src/plain.rs").unwrap(), Verbatim));
        match c_style_unquote(r#""src/we\tird\\\303\251.rs""#).unwrap() {
            Decoded(bytes) => {
                assert_eq!(String::from_utf8(bytes).unwrap(), "src/we\tird\\\u{e9}.rs")
            }
            Verbatim => panic!("quoted token must decode"),
        }
        assert!(c_style_unquote(r#""dangling\.rs"#).is_err());
        assert!(c_style_unquote(r#""bad \q escape""#).is_err());
        assert!(c_style_unquote(r#""truncated \12""#).is_err());
        assert!(c_style_unquote("\"unterminated").is_err());
    }
}
