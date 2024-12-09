use actix_web::{get, http::StatusCode, web, HttpResponse, HttpResponseBuilder};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::SearcherBuilder;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::{
    books::{Exclude, FilterMode, Include, RootBookDir, SearchResults},
    config::get_config,
    errors::{BadRequestError, InternalServerErrors, RegexProblem},
};

/// Represents parameters that determine the way
/// a search is made.
#[derive(Debug, ToSchema, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SearchForm {
    pattern: String,
    after_context: Option<usize>,
    before_context: Option<usize>,
    case_insensitive: Option<bool>,
    case_smart: Option<bool>,
    include_tags: Option<Vec<String>>,
    include_mode: Option<FilterMode>,
    exclude_tags: Option<Vec<String>>,
    exclude_mode: Option<FilterMode>,
}
/// Searches books filtered by tags.
#[utoipa::path(
    params(SearchForm),
    responses (
        (status = 200, body=[SearchResults]),
        (status = 400, content((BadRequestError))),
        (status = 500, content((InternalServerErrors))),
    )
)]
#[get("/search")]
pub async fn search(form: web::Query<SearchForm>) -> HttpResponse {
    let config = get_config();
    let searcher = SearcherBuilder::new()
        .after_context(form.after_context.unwrap_or_default())
        .before_context(form.before_context.unwrap_or_default())
        .build();
    let matcher = match RegexMatcherBuilder::new()
        .case_insensitive(form.case_insensitive.unwrap_or(false))
        .case_smart(form.case_smart.unwrap_or(false))
        .build(form.pattern.as_str())
    {
        Ok(v) => v,
        Err(e) => return RegexProblem::new(e).into(),
    };
    let root = RootBookDir::new(config.book_path);
    //TODO: maybe there is a way to remove those .clone()'s?
    let include = Include {
        mode: form.include_mode.clone().unwrap_or_default(),
        tags: form
            .include_tags
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect(),
    };
    let exclude = Exclude {
        mode: form.exclude_mode.clone().unwrap_or_default(),
        tags: form
            .exclude_tags
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect(),
    };
    let search_results = match root.search_by_tags(include, exclude, searcher, matcher) {
        Ok(v) => v,
        Err(e) => return e.into(),
    };
    HttpResponseBuilder::new(StatusCode::OK)
        .content_type("application/json")
        .json(search_results)
}
