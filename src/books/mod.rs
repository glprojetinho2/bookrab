mod history;
mod test_utils;
mod utils;
use crate::{
    config::BookrabConfig,
    errors::{GrepSearchError, InexistentBook},
};
use anyhow::anyhow;
use core::str;
use grep_matcher::{Match, Matcher};
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, Sink, SinkContextKind};
use history::SearchHistory;
use log::error;
use std::{collections::HashSet, fs, io};
use utils::{find_iter_at_in_context_single_line, from_utf8};
use utoipa::ToSchema;

use crate::errors::{
    BookrabError, CouldntCreateDir, CouldntReadChild, CouldntReadDir, CouldntReadFile,
    CouldntWriteFile, InvalidTags,
};

/// Represents elements returned by the listing
/// route.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema, PartialEq)]
pub struct BookListElement {
    /// Book title
    title: String,
    /// Book metadata for filtering
    tags: HashSet<String>,
}

/// Manages the way that books will be filtered by tags.
#[derive(Clone, Debug, ToSchema, Default, serde::Deserialize)]
pub enum FilterMode {
    /// Grabs books that have all of the tags.
    All,
    /// Grabs books that have any of the tags.
    #[default]
    Any,
}

/// Excludes matched books
#[derive(Clone, Debug, Default)]
pub struct Exclude {
    pub mode: FilterMode,
    pub tags: HashSet<String>,
}
/// Include matched books
#[derive(Clone, Debug)]
pub struct Include {
    pub mode: FilterMode,
    pub tags: HashSet<String>,
}

/// Associates search results with the title of a book.
#[derive(Clone, Debug, PartialEq, ToSchema, serde::Serialize)]
pub struct SearchResults {
    title: String,
    results: Vec<String>,
}

impl SearchResults {
    /// Generates a BookSink instance that can
    /// fill this instance with search results.
    fn sink<T: Matcher>(&mut self, matcher: T) -> BookSink<T> {
        BookSink::new(self, matcher)
    }
    fn new(title: String) -> Self {
        SearchResults {
            title,
            results: vec![],
        }
    }
}

/// Sink to be used in book searches.
/// It doesn't support passthru.
pub struct BookSink<'a, T: Matcher> {
    results: &'a mut SearchResults,
    matcher: T,
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
    fn new(results: &mut SearchResults, matcher: T) -> BookSink<T> {
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
    /// ```
    /// results == ["not last", "last"]
    /// this_func(" string".bytes())
    /// results == ["not last", "last string"]
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

/// Represents a root book folder.
/// In this folder we are going to store texts and metadata
/// in the way explained bellow:
/// ```
/// path/to/root_book_dir/ <= this is the `path` we use in this struct
/// ├─ book_title1/ <= folder with the book's title as its name
/// │  ├─ txt <= full text of the book
/// │  ├─ tags.json <= json in the format `["tag1", "tag2", ...]`
/// ├─ book_title2/
/// │  ├─ txt
/// │  ├─ tags.json
/// ```
#[derive(Debug)]
pub struct RootBookDir {
    config: BookrabConfig,
}

impl RootBookDir {
    const INFO_PATH: &'static str = "tags.json";
    pub fn new(config: BookrabConfig) -> RootBookDir {
        RootBookDir { config }
    }

    /// Gets book according to its title.
    pub fn get_by_title(&self, title: String) -> Result<Option<BookListElement>, BookrabError> {
        let list = self.list()?;
        let result: Vec<BookListElement> = list
            .into_iter()
            .filter(|book| book.title == title)
            .collect();
        // there are not going to be any duplicates
        Ok(result.into_iter().next())
    }

    /// Lists books according to their tags.
    /// No included tags = include all tags.
    /// No excluded tags = exclude no tags.
    /// These apply regardless of the mode of the inclusion/exclusion.
    pub fn list_by_tags(
        &self,
        include: Include,
        exclude: Exclude,
    ) -> Result<Vec<BookListElement>, BookrabError> {
        let list = self.list()?;
        let result = list
            .into_iter()
            .filter(|book| {
                let includes = if !include.tags.is_empty() {
                    match include.mode {
                        FilterMode::Any => !include
                            .tags
                            .intersection(&book.tags)
                            .collect::<Vec<&String>>()
                            .is_empty(),
                        FilterMode::All => {
                            include.tags.union(&book.tags).collect::<Vec<_>>().len()
                                == book.tags.len()
                        }
                    }
                } else {
                    true
                };
                let excludes = if !exclude.tags.is_empty() {
                    match exclude.mode {
                        FilterMode::Any => !exclude
                            .tags
                            .intersection(&book.tags)
                            .collect::<Vec<&String>>()
                            .is_empty(),
                        FilterMode::All => {
                            exclude.tags.union(&book.tags).collect::<Vec<_>>().len()
                                == book.tags.len()
                        }
                    }
                } else {
                    false
                };
                includes && !excludes
            })
            .collect();
        Ok(result)
    }

    /// Lists all books in the form of [BookListElement]
    pub fn list(&self) -> Result<Vec<BookListElement>, BookrabError> {
        let books_dir = match fs::read_dir(self.config.book_path.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("{e:#?}");
                return Err(BookrabError::CouldntReadDir(
                    CouldntReadDir::new(&self.config.book_path),
                    anyhow!(e),
                ));
            }
        };
        let mut result = vec![];
        for book_dir_res in books_dir {
            let book_dir = match book_dir_res {
                Ok(v) => v,
                Err(e) => {
                    return {
                        error!("{:#?}", e);
                        Err(BookrabError::CouldntReadChild(
                            CouldntReadChild::new(
                                self.config
                                    .book_path
                                    .to_str()
                                    .unwrap_or("path is not even valid unicode"),
                            ),
                            anyhow!(e),
                        ))
                    }
                }
            };
            let book_title = book_dir.file_name().to_str().unwrap().to_string();

            // extract metadata
            let tags_path = book_dir.path().join(Self::INFO_PATH);
            let tags_contents = if tags_path.exists() {
                match fs::read_to_string(&tags_path) {
                    Ok(v) => v,
                    Err(e) => {
                        return {
                            error!("{e:#?}");
                            Err(BookrabError::CouldntReadFile(
                                CouldntReadFile::new(&tags_path),
                                anyhow!(e),
                            ))
                        }
                    }
                }
            } else {
                let _ = fs::write(&tags_path, "[]");
                "[]".to_string()
            };
            let tags: HashSet<String> = match serde_json::from_str(tags_contents.as_str()) {
                Ok(v) => v,
                Err(e) => {
                    return {
                        error!("{:#?}", e);
                        Err(BookrabError::InvalidTags(InvalidTags::new(
                            tags_contents.as_str(),
                            &tags_path,
                        )))
                    }
                }
            };

            result.push(BookListElement {
                title: book_title,
                tags,
            });
        }

        Ok(result)
    }

    /// Uploads a single book.
    /// If the book is already there (i.e root_dir/title exists),
    /// the txt and tags are updated.
    pub fn upload(
        &self,
        title: &str,
        txt: &str,
        tags: HashSet<String>,
    ) -> Result<&Self, BookrabError> {
        // create book directory if it doesn't exist
        let book_path = &self.config.book_path.join(title);
        if let Err(e) = fs::create_dir_all(book_path) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(BookrabError::CouldntCreateDir(
                    CouldntCreateDir::new(book_path),
                    anyhow!(e),
                ));
            }
        }
        // write text
        let txt_path = book_path.join("txt");
        if let Err(e) = fs::write(&txt_path, txt) {
            return Err(BookrabError::CouldntWriteFile(
                CouldntWriteFile::new(&txt_path),
                anyhow!(e),
            ));
        };

        // write metadata
        let tags_str =
            serde_json::to_string(&tags).expect("BookTags could not be converted to string");
        let tags_path = book_path.join(Self::INFO_PATH);
        if let Err(e) = fs::write(&tags_path, tags_str) {
            return Err(BookrabError::CouldntWriteFile(
                CouldntWriteFile::new(&tags_path),
                anyhow!(e),
            ));
        };
        Ok(self)
    }

    /// Searches stuff in a single book.
    /// The search is configurable via parameters passed
    /// to the searcher (after_context, for example) or to the
    /// matcher (case_insensitive, for example).
    pub fn search(
        &self,
        title: String,
        mut searcher: Searcher,
        matcher: RegexMatcher,
    ) -> Result<SearchResults, BookrabError> {
        let mut results = SearchResults::new(title.clone());
        let book_path = self.config.book_path.join(title).join("txt");
        let sink = &mut results.sink(matcher);
        if book_path.exists() {
            if let Err(e) = searcher.search_path(sink.matcher.clone(), &book_path, sink) {
                return Err(BookrabError::GrepSearchError(
                    GrepSearchError::new(&book_path),
                    anyhow!(e),
                ));
            };
        } else {
            return Err(BookrabError::InexistentBook(InexistentBook::new(
                &book_path,
            )));
        }
        let res = SearchHistory::new(self.config.clone()).register_history(vec![results])?;
        Ok(res.first().unwrap().to_owned())
    }

    /// Searches stuff in all books that respect some
    /// tag constraint. See [RootBookDir::list_by_tags].
    pub fn search_by_tags(
        &self,
        include: Include,
        exclude: Exclude,
        searcher: Searcher,
        matcher: RegexMatcher,
    ) -> Result<Vec<SearchResults>, BookrabError> {
        let book_list = self.list_by_tags(include, exclude)?;
        let mut search_results = vec![];
        for book in book_list {
            let title = book.title;
            let single_search = self.search(title, searcher.clone(), matcher.clone())?;
            search_results.push(single_search);
        }
        SearchHistory::new(self.config.clone()).register_history(search_results)
    }
}

#[cfg(test)]
mod tests {
    use crate::books::RootBookDir;
    use chrono::Datelike;
    use chrono::Utc;
    use grep_regex::RegexMatcherBuilder;
    use grep_searcher::SearcherBuilder;
    use test_utils::{basic_metadata, create_book_dir, root_for_tag_tests, s, LUSIADAS1};

    use super::*;

    #[test]
    fn basic_uploading() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        let expected_text = "As armas e os barões assinalados";
        book_dir
            .upload("lusiadas", expected_text, basic_metadata())
            .unwrap();

        let txt = fs::read_to_string(book_dir.config.book_path.join("lusiadas").join("txt"))
            .expect("couldnt read txt (file not created?)");
        let tags_txt = fs::read_to_string(
            book_dir
                .config
                .book_path
                .join("lusiadas")
                .join(RootBookDir::INFO_PATH),
        )
        .expect("couldnt read info (file not created?)");
        let tags: HashSet<String> = serde_json::from_str(&tags_txt).unwrap();
        assert_eq!(txt, expected_text);
        assert_eq!(tags, basic_metadata());
        Ok(())
    }
    #[test]
    fn overwriting_with_upload() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        let expected_text = "As armas e os barões assinalados";
        book_dir
            .upload(
                "lusiadas",
                "whatever",
                HashSet::from(["whatever".to_string()]),
            )
            .unwrap();
        book_dir
            .upload("lusiadas", expected_text, basic_metadata())
            .unwrap();

        let txt = fs::read_to_string(book_dir.config.book_path.join("lusiadas").join("txt"))
            .expect("couldnt read txt (file not created?)");
        let tags_txt = fs::read_to_string(
            book_dir
                .config
                .book_path
                .join("lusiadas")
                .join(RootBookDir::INFO_PATH),
        )
        .expect("couldnt read info (file not created?)");
        let tags: HashSet<String> = serde_json::from_str(&tags_txt).unwrap();
        assert_eq!(txt, expected_text);
        assert_eq!(tags, basic_metadata());
        Ok(())
    }
    #[test]
    fn basic_listing() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        let body = book_dir.list().unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(
            body[0],
            BookListElement {
                title: "lusiadas".to_string(),
                tags: basic_metadata(),
            }
        );
        Ok(())
    }

    #[test]
    fn list_two_items() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        book_dir.upload("sonetos", "", basic_metadata()).unwrap();

        let body = book_dir.list().unwrap();
        assert_eq!(body.len(), 2);
        assert_eq!(
            body[0],
            BookListElement {
                title: "lusiadas".to_string(),
                tags: basic_metadata(),
            }
        );
        assert_eq!(
            body[1],
            BookListElement {
                title: "sonetos".to_string(),
                tags: basic_metadata(),
            }
        );
        Ok(())
    }

    #[test]
    fn list_invalid_metadata() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        let metadata_path = book_dir
            .config
            .book_path
            .join("lusiadas")
            .join(RootBookDir::INFO_PATH);
        fs::write(&metadata_path, "meeeeeeeeeeeeeeeeeeeessed up").unwrap();

        match book_dir.list().unwrap_err() {
            BookrabError::InvalidTags(err) => {
                assert_eq!(err.tags, "meeeeeeeeeeeeeeeeeeeessed up");
                assert_eq!(err.path, metadata_path.to_string_lossy());
            }
            _ => return Err(anyhow!("isnt invalid metadata")),
        }
        Ok(())
    }
    macro_rules! test_filter {
        ($include:expr, $exclude: expr, $expected: expr) => {{
            let book_dir = dbg!(root_for_tag_tests());
            let books = book_dir.list_by_tags($include, $exclude).unwrap();

            let expected = $expected;
            assert_eq!(books.len(), expected.len());
            assert_eq!(
                books
                    .iter()
                    .map(|book| book.title.clone())
                    .collect::<HashSet<_>>(),
                expected
            );
            (book_dir, books)
        }};
    }

    #[test]
    fn filter_include_all() -> Result<(), anyhow::Error> {
        test_filter!(
            Include {
                mode: FilterMode::All,
                tags: s(vec!["d", "c"])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec![]),
            },
            s(vec!["1"])
        );
        Ok(())
    }
    #[test]
    fn filter_include_any() -> Result<(), anyhow::Error> {
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec!["d", "c"])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec![]),
            },
            s(vec!["1", "2"])
        );
        Ok(())
    }
    #[test]
    fn filter_exclude_all() -> Result<(), anyhow::Error> {
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec![])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec!["d", "c"]),
            },
            s(vec!["2", "3", "4"])
        );
        Ok(())
    }
    #[test]
    fn filter_exclude_any() -> Result<(), anyhow::Error> {
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec![])
            },
            Exclude {
                mode: FilterMode::Any,
                tags: s(vec!["d", "c"]),
            },
            s(vec!["3", "4"])
        );
        Ok(())
    }
    #[test]
    fn filter_include_any_exclude_any() -> Result<(), anyhow::Error> {
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec!["b"])
            },
            Exclude {
                mode: FilterMode::Any,
                tags: s(vec!["d", "c"]),
            },
            s(vec!["3"])
        );
        Ok(())
    }
    #[test]
    fn filter_include_all_exclude_all() -> Result<(), anyhow::Error> {
        test_filter!(
            Include {
                mode: FilterMode::All,
                tags: s(vec!["b"])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec!["b", "d"]),
            },
            s(vec!["2", "3"])
        );
        Ok(())
    }
    #[test]
    fn filter_include_any_exclude_all() -> Result<(), anyhow::Error> {
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec!["c", "d", "b"])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec!["a", "d"]),
            },
            s(vec!["2", "3"])
        );
        Ok(())
    }
    #[test]
    fn filter_include_all_exclude_any() -> Result<(), anyhow::Error> {
        test_filter!(
            Include {
                mode: FilterMode::All,
                tags: s(vec!["c", "d", "b"])
            },
            Exclude {
                mode: FilterMode::Any,
                tags: s(vec!["a", "d"]),
            },
            s(vec![])
        );
        Ok(())
    }

    #[test]
    fn get_by_title() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        let book = book_dir.get_by_title("lusiadas".to_string())?.unwrap();
        assert_eq!(
            book,
            BookListElement {
                title: "lusiadas".to_string(),
                tags: basic_metadata(),
            }
        );
        Ok(())
    }

    macro_rules! test_search {
        ($name:ident, $searcher: expr, $matcher: expr, $expected_results: expr) => {
            #[test]
            fn $name() -> Result<(), anyhow::Error> {
                let book_dir = create_book_dir();
                book_dir
                    .upload("lusiadas", LUSIADAS1, basic_metadata())
                    .unwrap();
                let result = book_dir
                    .search(String::from("lusiadas"), $searcher, $matcher)
                    .unwrap();
                assert_eq!(result.title, "lusiadas");
                assert_eq!(result.results, $expected_results);
                let history_str = fs::read_to_string(book_dir.config.history_path).unwrap();
                let now = Utc::now();
                assert!(history_str.contains("lusiadas"));
                assert!(history_str.contains("[matched]"));
                assert!(history_str.contains(now.year().to_string().as_str()));
                Ok(())
            }
        };
    }
    test_search!(
        basic_search,
        SearcherBuilder::new().build(),
        RegexMatcher::new(r"\bpadeceu\b").unwrap(),
        vec!["Que [matched]padeceu[/matched] desonra e vitupério,\n"]
    );

    test_search!(
        multiple_results_in_one_line_search,
        SearcherBuilder::new().build(),
        RegexMatcher::new(r"v").unwrap(),
        vec![
            "Obedece o [matched]v[/matched]isíbil e ín[matched]v[/matched]isíbil\n",
            "Que padeceu desonra e [matched]v[/matched]itupério,\n",
            "Os li[matched]v[/matched]ros, que tu pedes não trazia,\n",
            "Em papel o que na alma andar de[matched]v[/matched]ia.\n",
            "Se as armas queres [matched]v[/matched]er, como tens dito,\n",
            "Como amigo as [matched]v[/matched]erás; porque eu me obrigo,\n",
            "Que nunca as queiras [matched]v[/matched]er como inimigo.\n",
            "Arcos, e sagitíferas alja[matched]v[/matched]as,\n",
            "Partazanas agudas, chuças bra[matched]v[/matched]as:"
        ]
    );

    test_search!(
        search_with_after_context,
        SearcherBuilder::new().after_context(2).build(),
        RegexMatcherBuilder::new()
            .case_insensitive(true)
            .build(r"\bpor\w*?")
            .unwrap(),
        vec![
            "[matched]Por[/matched] subir os mortais da Terra ao Céu.\n\nDeste Deus-Homem, alto e infinito,\n",
            "Como amigo as verás; [matched]por[/matched]que eu me obrigo,\nQue nunca as queiras ver como inimigo.\n\n"
        ]
    );
    test_search!(
        search_with_before_context,
        SearcherBuilder::new().before_context(2).build(),
        RegexMatcherBuilder::new()
            .case_insensitive(true)
            .build(r"\bpor\w*?")
            .unwrap(),
            vec![
                "Sofrendo morte injusta e insofríbil,\nE que do Céu à Terra, enfim desceu,\n[matched]Por[/matched] subir os mortais da Terra ao Céu.\n", 
                "Se as armas queres ver, como tens dito,\nCumprido esse desejo te seria;\nComo amigo as verás; [matched]por[/matched]que eu me obrigo,\n"
            ]
    );
    test_search!(
        search_with_both_contexts,
        SearcherBuilder::new()
            .before_context(1)
            .after_context(1)
            .build(),
        RegexMatcherBuilder::new()
            .case_insensitive(true)
            .build(r"\bpor\w*?")
            .unwrap(),
        vec!["E que do Céu à Terra, enfim desceu,\n[matched]Por[/matched] subir os mortais da Terra ao Céu.\n\n", "Cumprido esse desejo te seria;\nComo amigo as verás; [matched]por[/matched]que eu me obrigo,\nQue nunca as queiras ver como inimigo.\n"]
    );

    #[test]
    fn search_by_tags() -> Result<(), anyhow::Error> {
        let include = Include {
            mode: FilterMode::Any,
            tags: s(vec!["c", "d", "b"]),
        };
        let exclude = Exclude {
            mode: FilterMode::All,
            tags: s(vec!["a", "d"]),
        };
        let (book_dir, _books) = test_filter!(include.clone(), exclude.clone(), s(vec!["2", "3"]));
        let searcher = SearcherBuilder::new()
            .before_context(1)
            .after_context(1)
            .build();
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(true)
            .build(r"\bpor\w*?")
            .unwrap();
        let search_results = book_dir
            .search_by_tags(include, exclude, searcher, matcher)
            .unwrap();
        assert_eq!(search_results,
        vec![
    SearchResults {
        title: String::from("2"),
        results: vec![
            "Que da ocidental praia Lusitana,\n[matched]Por[/matched] mares nunca de antes navegados,\nPassaram ainda além da Taprobana,\n".to_string(),
            "De África e de Ásia andaram devastando;\nE aqueles, que [matched]por[/matched] obras valerosas\nSe vão da lei da morte libertando;\n".to_string(),
            "Cantando espalharei [matched]por[/matched] toda parte,\nSe a tanto me ajudar o engenho e arte.\n".to_string(),
        ],
    },
    SearchResults {
        title: String::from("3"),
        results: vec![
            "Menos trabalho em tal negócio gasta:\nAta o cordão que traz, [matched]por[/matched] derradeiro,\nNo tronco, e fàcilmente o leva e arrasta\n".to_string(),
            "Pera onde faça um sumptuoso templo\nQue ficasse aos futuros [matched]por[/matched] exemplo.\n\n".to_string(),
            "A gente ficou disto alvoraçada;\nOs Brâmenes o têm [matched]por[/matched] cousa nova;\nVendo os milagres, vendo a santidade,\n".to_string(),
        ],
    },
]
    );
        Ok(())
    }
}
