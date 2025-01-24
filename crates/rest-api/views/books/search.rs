use crate::{
    config::ensure_confy_works,
    database::DB,
    errors::{ApiError, Bookrab400, Bookrab500},
};
use actix_web::{get, http::StatusCode, web, HttpResponse, HttpResponseBuilder};
use bookrab_core::books::{Exclude, FilterMode, Include, RootBookDir};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::SearcherBuilder;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, ToSchema)]
struct SearchResultsUtoipa {
    title: String,
    results: Vec<String>,
}

/// Represents parameters that determine the way
/// a search is made.
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize, ToSchema)]
enum FilterModeUtoipa {
    All,
    Any,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SearchFormUtoipa {
    after_context: Option<usize>,
    before_context: Option<usize>,
    case_insensitive: Option<bool>,
    case_smart: Option<bool>,
    exclude_mode: Option<FilterModeUtoipa>,
    exclude_tags: Option<Vec<String>>,
    include_mode: Option<FilterModeUtoipa>,
    include_tags: Option<Vec<String>>,
    pattern: String,
}

/// Searches books filtered by tags.
#[utoipa::path(
    params(SearchFormUtoipa),
    responses (
        (status = 200, body=[SearchResultsUtoipa]),
        (status = 400, body=Bookrab400),
        (status = 500, body=Bookrab500),
    )
)]
#[get("/search")]
pub async fn search(form: web::Query<SearchForm>, mut db: DB) -> HttpResponse {
    let config = ensure_confy_works();
    let searcher = SearcherBuilder::new()
        .after_context(form.after_context.unwrap_or_default())
        .before_context(form.before_context.unwrap_or_default())
        .build();
    let mut builder = RegexMatcherBuilder::new();
    let matcher_builder = builder
        .case_insensitive(form.case_insensitive.unwrap_or(false))
        .case_smart(form.case_smart.unwrap_or(false));
    let mut root = RootBookDir::new(config, &mut db.connection);
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
    let search_results = match root.search_by_tags(
        &include,
        &exclude,
        form.pattern.clone(),
        searcher,
        matcher_builder.clone(),
    ) {
        Ok(v) => v,
        Err(e) => return ApiError(e).into(),
    };
    HttpResponseBuilder::new(StatusCode::OK)
        .content_type("application/json")
        .json(search_results)
}
