use std::io;

use grep_matcher::Match;
use {
    grep_matcher::Matcher,
    grep_searcher::{Searcher, SinkError},
};

/// Given a buf and some bounds, if there is a line terminator at the end of
/// the given bounds in buf, then the bounds are trimmed to remove the line
/// terminator.
pub(crate) fn trim_line_terminator(searcher: &Searcher, buf: &[u8], line: &mut Match) {
    let lineterm = searcher.line_terminator();
    if lineterm.is_suffix(&buf[*line]) {
        let mut end = line.end() - 1;
        if lineterm.is_crlf() && end > 0 && buf.get(end - 1) == Some(&b'\r') {
            end -= 1;
        }
        *line = line.with_end(end);
    }
}

pub(crate) fn find_iter_at_in_context_single_line<M, F>(
    searcher: &Searcher,
    matcher: M,
    mut bytes: &[u8],
    range: std::ops::Range<usize>,
    mut matched: F,
) -> io::Result<()>
where
    M: Matcher,
    F: FnMut(Match) -> bool,
{
    // When searching a single line, we should remove the line terminator.
    // Otherwise, it's possible for the regex (via look-around) to observe
    // the line terminator and not match because of it.
    let mut m = Match::new(0, range.end);
    trim_line_terminator(searcher, bytes, &mut m);
    bytes = &bytes[..m.end()];
    matcher
        .find_iter_at(bytes, range.start, |m| {
            if m.start() >= range.end {
                return false;
            }
            matched(m)
        })
        .map_err(io::Error::error_message)
}

pub(crate) fn from_utf8(bytes: &[u8]) -> Result<&str, std::io::Error> {
    match std::str::from_utf8(bytes) {
        Ok(matched) => Ok(matched),
        Err(err) => return Err(std::io::Error::error_message(err)),
    }
}
