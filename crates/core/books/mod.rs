mod history;
mod sink;
mod test_utils;
mod utils;

use crate::{config::BookrabConfig, database::PgPooledConnection};
use core::str;
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::Searcher;
use history::SearchHistory;
use log::error;
use sink::BookSink;
use std::{collections::HashSet, fs};

use crate::errors::BookrabError;

/// Represents elements returned by the listing
/// route.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct BookListElement {
    /// Book title
    title: String,
    /// Book metadata for filtering
    tags: HashSet<String>,
}

/// Manages the way that books will be filtered by tags.
#[derive(Clone, Debug, Default, serde::Deserialize)]
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
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
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

/// Represents a root book folder.
/// In this folder we are going to store texts and metadata
/// in the way explained bellow:
/// ```no_compile
/// path/to/root_book_dir/ <= this is the `path` we use in this struct
/// ├─ book_title1/ <= folder with the book's title as its name
/// │  ├─ txt <= full text of the book
/// │  ├─ tags.json <= json in the format `["tag1", "tag2", ...]`
/// ├─ book_title2/
/// │  ├─ txt
/// │  ├─ tags.json
/// ```
pub struct RootBookDir<'a> {
    config: BookrabConfig,
    /// Connection to Postgresql
    pub connection: &'a mut PgPooledConnection,
}

impl<'a> RootBookDir<'a> {
    const INFO_PATH: &'static str = "tags.json";
    pub fn new(config: BookrabConfig, connection: &mut PgPooledConnection) -> RootBookDir {
        RootBookDir { config, connection }
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
        let books_dir = match fs::read_dir(&self.config.book_path) {
            Ok(v) => v,
            Err(e) => {
                error!("{e:#?}");
                return Err(BookrabError::CouldntReadDir {
                    error: (),
                    path: self.config.book_path.clone(),
                    err: e,
                });
            }
        };
        let mut result = vec![];
        for book_dir_res in books_dir {
            let book_dir = match book_dir_res {
                Ok(v) => v,
                Err(e) => {
                    return Err(BookrabError::CouldntReadChild {
                        error: (),
                        parent: self.config.book_path.clone(),
                        err: e,
                    })
                }
            };
            let book_title = book_dir.file_name().to_str().unwrap().to_string();

            // extract metadata
            let tags_path = book_dir.path().join(Self::INFO_PATH);
            let tags_contents = if tags_path.exists() {
                match fs::read_to_string(&tags_path) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(BookrabError::CouldntReadFile {
                            error: (),
                            path: tags_path,
                            err: e,
                        })
                    }
                }
            } else {
                let _ = fs::write(&tags_path, "[]");
                "[]".to_string()
            };
            let tags: HashSet<String> = match serde_json::from_str(tags_contents.as_str()) {
                Ok(v) => v,
                Err(e) => {
                    return Err(BookrabError::InvalidTags {
                        error: (),
                        tags: tags_contents,
                        path: tags_path,
                        err: e,
                    })
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
                return Err(BookrabError::CouldntCreateDir {
                    error: (),
                    path: book_path.to_owned(),
                    err: e,
                });
            }
        }
        // write text
        let txt_path = book_path.join("txt");
        if let Err(e) = fs::write(&txt_path, txt) {
            return Err(BookrabError::CouldntWriteFile {
                error: (),
                path: txt_path,
                err: e,
            });
        };

        // write metadata
        let tags_str =
            serde_json::to_string(&tags).expect("BookTags could not be converted to string");
        let tags_path = book_path.join(Self::INFO_PATH);
        if let Err(e) = fs::write(&tags_path, tags_str) {
            return Err(BookrabError::CouldntWriteFile {
                error: (),
                path: tags_path,
                err: e,
            });
        };
        Ok(self)
    }

    /// Searches stuff in a single book.
    /// The search is configurable via parameters passed
    /// to the searcher (after_context, for example) or to the
    /// matcher (case_insensitive, for example).
    pub fn search(
        &mut self,
        title: String,
        // we have to pass a pattern and a builder to this function
        // because there is no way to extract the pattern from a
        // RegexMatcher (AFAIK).
        pattern: String,
        mut searcher: Searcher,
        matcher_builder: RegexMatcherBuilder,
    ) -> Result<SearchResults, BookrabError> {
        let matcher = matcher_builder.build(pattern.as_str())?;
        let mut results = SearchResults::new(title.clone());
        let book_path = self.config.book_path.join(title).join("txt");
        let sink = &mut results.sink(matcher);
        if book_path.exists() {
            if let Err(e) = searcher.search_path(sink.matcher.clone(), &book_path, sink) {
                return Err(BookrabError::GrepSearchError {
                    error: (),
                    path: book_path,
                    err: e,
                });
            };
        } else {
            return Err(BookrabError::InexistentBook {
                error: (),
                path: book_path,
            });
        }
        let results_vec = vec![results];
        let search_history = SearchHistory::new(self.config.clone(), self.connection);
        let res = search_history.register_history(pattern, &results_vec)?;
        Ok(res.first().unwrap().to_owned())
    }

    /// Searches stuff in all books that respect some
    /// tag constraint. See [RootBookDir::list_by_tags].
    /// This also generates history entries.
    pub fn search_by_tags(
        &mut self,
        include: Include,
        exclude: Exclude,
        pattern: String,
        searcher: Searcher,
        matcher_builder: RegexMatcherBuilder,
    ) -> Result<Vec<SearchResults>, BookrabError> {
        let book_list = self.list_by_tags(include, exclude)?;
        let mut search_results = vec![];
        for book in book_list {
            let title = book.title;
            let single_search = self.search(
                title,
                pattern.clone(),
                searcher.clone(),
                matcher_builder.clone(),
            )?;
            search_results.push(single_search.to_owned());
        }
        let search_history = SearchHistory::new(self.config.clone(), self.connection);
        let res = search_history.register_history(pattern, &search_results)?;
        Ok(res.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use crate::books::test_utils::DBCONNECTION;
    use crate::books::RootBookDir;
    use grep_regex::RegexMatcherBuilder;
    use grep_searcher::SearcherBuilder;
    use test_utils::{basic_metadata, create_book_dir, root_for_tag_tests, s, LUSIADAS1};

    use super::*;

    #[test]
    fn basic_uploading() -> Result<(), anyhow::Error> {
        let connection = &mut DBCONNECTION.get().unwrap();
        let book_dir = create_book_dir(connection);
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
        let connection = &mut DBCONNECTION.get().unwrap();
        let book_dir = create_book_dir(connection);
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
        let connection = &mut DBCONNECTION.get().unwrap();
        let book_dir = create_book_dir(connection);
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
        let connection = &mut DBCONNECTION.get().unwrap();
        let book_dir = create_book_dir(connection);
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
    fn list_invalid_metadata() -> Result<(), BookrabError> {
        let connection = &mut DBCONNECTION.get().unwrap();
        let book_dir = create_book_dir(connection);
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        let metadata_path = book_dir
            .config
            .book_path
            .join("lusiadas")
            .join(RootBookDir::INFO_PATH);
        fs::write(&metadata_path, "meeeeeeeeeeeeeeeeeeeessed up").unwrap();

        if let BookrabError::InvalidTags {
            error: (),
            tags,
            path,
            err: _err,
        } = book_dir.list().unwrap_err()
        {
            assert_eq!(tags, "meeeeeeeeeeeeeeeeeeeessed up");
            assert_eq!(path, metadata_path);
        } else {
            panic!("isnt invalid metadata");
        }

        Ok(())
    }
    macro_rules! test_filter {
        ($include:expr, $exclude: expr, $expected: expr, $connection: expr) => {{
            let book_dir = root_for_tag_tests($connection);
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
        let connection = &mut DBCONNECTION.get().unwrap();
        test_filter!(
            Include {
                mode: FilterMode::All,
                tags: s(vec!["d", "c"])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec![]),
            },
            s(vec!["1"]),
            connection
        );
        Ok(())
    }
    #[test]
    fn filter_include_any() -> Result<(), anyhow::Error> {
        let connection = &mut DBCONNECTION.get().unwrap();
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec!["d", "c"])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec![]),
            },
            s(vec!["1", "2"]),
            connection
        );
        Ok(())
    }
    #[test]
    fn filter_exclude_all() -> Result<(), anyhow::Error> {
        let connection = &mut DBCONNECTION.get().unwrap();
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec![])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec!["d", "c"]),
            },
            s(vec!["2", "3", "4"]),
            connection
        );
        Ok(())
    }
    #[test]
    fn filter_exclude_any() -> Result<(), anyhow::Error> {
        let connection = &mut DBCONNECTION.get().unwrap();
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec![])
            },
            Exclude {
                mode: FilterMode::Any,
                tags: s(vec!["d", "c"]),
            },
            s(vec!["3", "4"]),
            connection
        );
        Ok(())
    }
    #[test]
    fn filter_include_any_exclude_any() -> Result<(), anyhow::Error> {
        let connection = &mut DBCONNECTION.get().unwrap();
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec!["b"])
            },
            Exclude {
                mode: FilterMode::Any,
                tags: s(vec!["d", "c"]),
            },
            s(vec!["3"]),
            connection
        );
        Ok(())
    }
    #[test]
    fn filter_include_all_exclude_all() -> Result<(), anyhow::Error> {
        let connection = &mut DBCONNECTION.get().unwrap();
        test_filter!(
            Include {
                mode: FilterMode::All,
                tags: s(vec!["b"])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec!["b", "d"]),
            },
            s(vec!["2", "3"]),
            connection
        );
        Ok(())
    }
    #[test]
    fn filter_include_any_exclude_all() -> Result<(), anyhow::Error> {
        let connection = &mut DBCONNECTION.get().unwrap();
        test_filter!(
            Include {
                mode: FilterMode::Any,
                tags: s(vec!["c", "d", "b"])
            },
            Exclude {
                mode: FilterMode::All,
                tags: s(vec!["a", "d"]),
            },
            s(vec!["2", "3"]),
            connection
        );
        Ok(())
    }
    #[test]
    fn filter_include_all_exclude_any() -> Result<(), anyhow::Error> {
        let connection = &mut DBCONNECTION.get().unwrap();
        test_filter!(
            Include {
                mode: FilterMode::All,
                tags: s(vec!["c", "d", "b"])
            },
            Exclude {
                mode: FilterMode::Any,
                tags: s(vec!["a", "d"]),
            },
            s(vec![]),
            connection
        );
        Ok(())
    }

    #[test]
    fn get_by_title() -> Result<(), BookrabError> {
        let connection = &mut DBCONNECTION.get().unwrap();
        let book_dir = create_book_dir(connection);
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
        ($name:ident, $searcher: expr, $pattern: expr, $matcher_builder: expr, $expected_results: expr) => {
            #[test]
            fn $name() -> Result<(), anyhow::Error> {
                let connection = &mut DBCONNECTION.get().unwrap();
                let mut book_dir = create_book_dir(connection);
                book_dir
                    .upload("lusiadas", LUSIADAS1, basic_metadata())
                    .unwrap();
                let result = book_dir
                    .search(
                        String::from("lusiadas"),
                        $pattern,
                        $searcher,
                        $matcher_builder.clone(),
                    )
                    .unwrap();
                assert_eq!(result.title, "lusiadas");
                assert_eq!(result.results, $expected_results);
                Ok(())
            }
        };
    }
    test_search!(
        basic_search,
        SearcherBuilder::new().build(),
        r"\bpadeceu\b".to_string(),
        RegexMatcherBuilder::new(),
        vec!["Que [matched]padeceu[/matched] desonra e vitupério,\n"]
    );

    test_search!(
        multiple_results_in_one_line_search,
        SearcherBuilder::new().build(),
        r"v".to_string(),
        RegexMatcherBuilder::new(),
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
        r"\bpor\w*?".to_string(),
        RegexMatcherBuilder::new()
            .case_insensitive(true),
        vec![
            "[matched]Por[/matched] subir os mortais da Terra ao Céu.\n\nDeste Deus-Homem, alto e infinito,\n",
            "Como amigo as verás; [matched]por[/matched]que eu me obrigo,\nQue nunca as queiras ver como inimigo.\n\n"
        ]
    );
    test_search!(
        search_with_before_context,
        SearcherBuilder::new().before_context(2).build(),
            r"\bpor\w*?".to_string(),
        RegexMatcherBuilder::new()
            .case_insensitive(true),
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
            r"\bpor\w*?".to_string(),
        RegexMatcherBuilder::new()
            .case_insensitive(true),
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
        let connection = &mut DBCONNECTION.get().unwrap();
        let (mut book_dir, _books) = test_filter!(
            include.clone(),
            exclude.clone(),
            s(vec!["2", "3"]),
            connection
        );
        let searcher = SearcherBuilder::new()
            .before_context(1)
            .after_context(1)
            .build();
        let mut builder = RegexMatcherBuilder::new();
        let matcher_builder = builder.case_insensitive(true);
        let search_results = book_dir
            .search_by_tags(
                include,
                exclude,
                r"\bpor\w*?".to_string(),
                searcher,
                matcher_builder.clone(),
            )
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
