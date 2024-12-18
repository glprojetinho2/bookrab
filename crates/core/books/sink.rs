use super::{
    utils::{find_iter_at_in_context_single_line, from_utf8},
    SearchResults,
};
use grep_matcher::{Match, Matcher};
use grep_searcher::{Searcher, Sink, SinkContextKind};
use std::io;

/// Sink to be used in book searches.
/// It doesn't support passthru.
pub struct BookSink<'a, T: Matcher> {
    results: &'a mut SearchResults,
    pub(crate) matcher: T,
    matches: Vec<Match>,
    after_context_id: usize,
}

impl<T: Matcher> BookSink<'_, T> {
    /// Execute the matcher over the given bytes and record the match locations.
    fn record_matches(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.matches.clear();
        // If printing requires knowing the location of each individual match,
        // then compute and stored those right now for use later. While this
        // adds an extra copy for storing the matches, we do amortize the
        // allocation for it and this greatly simplifies the printing logic to
        // the extent that it's easy to ensure that we never do more than
        // one search to find the matches (well, for replacements, we do one
        // additional search to perform the actual replacement).
        let matches = &mut self.matches;
        find_iter_at_in_context_single_line(searcher, &self.matcher, bytes, range.clone(), |m| {
            let (s, e) = (m.start() - range.start, m.end() - range.start);
            matches.push(Match::new(s, e));
            true
        })?;
        // Don't report empty matches appearing at the end of the bytes.
        if !matches.is_empty()
            && matches.last().unwrap().is_empty()
            && matches.last().unwrap().start() >= range.end
        {
            matches.pop().unwrap();
        }
        Ok(())
    }

    /// Creates new [BookSink] instance from [SearchResults] instance
    pub fn new(results: &mut SearchResults, matcher: T) -> BookSink<T> {
        BookSink {
            results,
            matcher,
            matches: vec![],
            after_context_id: 0,
        }
    }
    /// Pushes string to the last entry in `self.results.results`.
    /// The string is obtained by converting `bytes` into UTF-8.
    /// Example in my pseudo-language:
    /// ```no_compile
    /// results == ["not last", "last"];
    /// this_func(" string".bytes());
    /// results == ["not last", "last string"];
    /// ```
    fn push_to_last_entry(&mut self, value: &str) -> Result<(), std::io::Error> {
        let mut current_result = self.results.results.pop().unwrap_or_default();
        current_result += value;
        self.results.results.push(current_result);
        Ok(())
    }
}
impl<T: Matcher> Sink for BookSink<'_, T> {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        searcher: &grep_searcher::Searcher,
        mat: &grep_searcher::SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        // Mathes are always appended to the last
        // entry of the results with `self.push_to_last_entry`.
        // If there is no after_context, then matches are treated the
        // same as the last contextual line of the `After` kind
        // (see the comment in the context function).

        // here we add [matched] [/matched] around the search result.
        self.record_matches(searcher, mat.buffer(), mat.bytes_range_in_buffer())?;
        let raw_result = from_utf8(mat.bytes())?;
        let mut result_with_matched_tags = String::from(raw_result);
        let opening_tag = "[matched]";
        let closing_tag = "[/matched]";
        for m in self.matches.iter() {
            let offset = result_with_matched_tags.len() - raw_result.len();
            let start = m.start() + offset;
            let end = m.end() + offset;
            let r = result_with_matched_tags;
            result_with_matched_tags = format!(
                "{}{}{}{}{}",
                &r[..start],
                opening_tag,
                &r[start..end],
                closing_tag,
                &r[end..]
            );
        }
        self.push_to_last_entry(result_with_matched_tags.as_str())?;
        if searcher.after_context() == 0 {
            self.results.results.push("".to_string());
        }

        Ok(true)
    }

    fn context(
        &mut self,
        searcher: &grep_searcher::Searcher,
        context: &grep_searcher::SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        // Context lines are always appended to the last
        // entry of the results with `self.push_to_last_entry`
        // If the function detects that this is the last `After` context,
        // it pushes an empty string to the results.
        // # Example
        // Let's say that the searcher has after_context = 2. In that case
        // the Sink is going to process data in the following way:
        // match comes in => results == ["match"]
        // first contextual line => results == ["match context1"]
        // second contextual line => results == ["match context1 context2", ""] <= observe the empty string
        // another match => results = ["match context1 context2", "another match"]
        // and so on.
        self.push_to_last_entry(from_utf8(context.bytes())?)?;
        if let SinkContextKind::After = context.kind() {
            self.after_context_id += 1;
            if self.after_context_id == searcher.after_context() {
                self.after_context_id = 0;
                self.results.results.push("".to_string());
            }
        }

        Ok(true)
    }
    fn finish(
        &mut self,
        _searcher: &Searcher,
        _: &grep_searcher::SinkFinish,
    ) -> Result<(), Self::Error> {
        // If the last element of `results` is an empty string,
        // (I believe this is always the case) then remove it.
        if self
            .results
            .results
            .last()
            .unwrap_or(&String::new())
            .is_empty()
        {
            self.results.results.pop();
        };
        Ok(())
    }
}
