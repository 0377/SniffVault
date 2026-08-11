use serde::Serialize;
use video_sniffing_engine::EngineError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FfiError {
    pub kind: String,
    pub message: String,
}

pub fn map_engine_error(err: EngineError) -> FfiError {
    match err {
        EngineError::Io(e) => FfiError {
            kind: "io".to_string(),
            message: e.to_string(),
        },
        EngineError::Db(e) => FfiError {
            kind: "db".to_string(),
            message: e.to_string(),
        },
        EngineError::Serde(e) => FfiError {
            kind: "serde".to_string(),
            message: e.to_string(),
        },
        EngineError::NotFound(msg) => FfiError {
            kind: "not_found".to_string(),
            message: msg,
        },
        EngineError::InvalidArg(msg) => FfiError {
            kind: "invalid_arg".to_string(),
            message: msg,
        },
        EngineError::Http(e) => FfiError {
            kind: "http".to_string(),
            message: e.to_string(),
        },
        EngineError::Message(msg) => FfiError {
            kind: "message".to_string(),
            message: msg,
        },
    }
}

#[derive(Serialize)]
struct OkResponse<T: Serialize> {
    ok: bool,
    data: T,
}

#[derive(Serialize)]
struct ErrResponse {
    ok: bool,
    error: FfiError,
}

pub fn ok_json<T: Serialize>(data: T) -> String {
    serde_json::to_string(&OkResponse { ok: true, data }).expect("serialize ok response")
}

pub fn err_json(err: EngineError) -> String {
    let error = map_engine_error(err);
    serde_json::to_string(&ErrResponse { ok: false, error }).expect("serialize err response")
}
