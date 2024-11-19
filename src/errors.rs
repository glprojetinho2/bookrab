#[macro_export]
macro_rules! api_error {
    (1, $path:expr, $err:expr) => {{
        let result = HttpResponse::InternalServerError().json(json!({ "error": "E0001: could not save file permanently.", "path": $path }));
        error!("{:#?}", $err);
        result
    }};
    (2, $path:expr, $err:expr) => {{
        let result = HttpResponse::InternalServerError().json(json!({ "error": "E0002: could not create directory.", "path": $path }));
        error!("{:#?}", $err);
        result
    }};
    (3, $filename:expr) => {{
        let result = HttpResponse::BadRequest().json(json!({ "error": "E0003: file should have 'text/plain' content type.", "filename": $filename}));
        result
    }};
    (4, $path:expr, $err:expr) => {{
        let result = HttpResponse::InternalServerError().json(json!({ "error": "E0004: could not write metadata.", "path": $path }));
        error!("{:#?}", $err);
        result
    }};
    (5, $path:expr, $err:expr) => {{
        let result = HttpResponse::InternalServerError().json(json!({ "error": "E0005: one of your book folders is messed up.", "path": $path }));
        error!("{:#?}", $err);
        result
    }};
    (6, $err:expr) => {{
        let result = HttpResponse::InternalServerError().json(json!({ "error": "E0006: couldnt read child of your book folder."}));
        error!("{:#?}", $err);
        result
    }};
    (7, $metadata:expr) => {{
        let result = HttpResponse::InternalServerError().json(json!({ "error": "E0007: invalid metadata.", "metadata": $metadata}));
        result
        }}
}
