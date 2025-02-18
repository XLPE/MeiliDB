use std::fmt::Display;

use http::status::StatusCode;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use tide::response::IntoResponse;
use tide::Response;

pub type SResult<T> = Result<T, ResponseError>;

pub enum ResponseError {
    Internal(String),
    BadRequest(String),
    InvalidToken(String),
    NotFound(String),
    IndexNotFound(String),
    DocumentNotFound(String),
    MissingHeader(String),
    BadParameter(String, String),
    CreateIndex(String),
    Maintenance,
}

impl ResponseError {
    pub fn internal(message: impl Display) -> ResponseError {
        ResponseError::Internal(message.to_string())
    }

    pub fn bad_request(message: impl Display) -> ResponseError {
        ResponseError::BadRequest(message.to_string())
    }

    pub fn invalid_token(message: impl Display) -> ResponseError {
        ResponseError::InvalidToken(message.to_string())
    }

    pub fn not_found(message: impl Display) -> ResponseError {
        ResponseError::NotFound(message.to_string())
    }

    pub fn index_not_found(message: impl Display) -> ResponseError {
        ResponseError::IndexNotFound(message.to_string())
    }

    pub fn document_not_found(message: impl Display) -> ResponseError {
        ResponseError::DocumentNotFound(message.to_string())
    }

    pub fn missing_header(message: impl Display) -> ResponseError {
        ResponseError::MissingHeader(message.to_string())
    }

    pub fn bad_parameter(name: impl Display, message: impl Display) -> ResponseError {
        ResponseError::BadParameter(name.to_string(), message.to_string())
    }

    pub fn create_index(message: impl Display) -> ResponseError {
        ResponseError::CreateIndex(message.to_string())
    }
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> Response {
        match self {
            ResponseError::Internal(err) => {
                error!("internal server error: {}", err);
                error(
                    String::from("Internal server error"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
            ResponseError::BadRequest(err) => {
                warn!("bad request: {}", err);
                error(err, StatusCode::BAD_REQUEST)
            }
            ResponseError::InvalidToken(err) => {
                error(format!("Invalid Token: {}", err), StatusCode::FORBIDDEN)
            }
            ResponseError::NotFound(err) => error(err, StatusCode::NOT_FOUND),
            ResponseError::IndexNotFound(index) => {
                error(format!("Index {} not found", index), StatusCode::NOT_FOUND)
            }
            ResponseError::DocumentNotFound(id) => error(
                format!("Document with id {} not found", id),
                StatusCode::NOT_FOUND,
            ),
            ResponseError::MissingHeader(header) => error(
                format!("Header {} is missing", header),
                StatusCode::UNAUTHORIZED,
            ),
            ResponseError::BadParameter(param, e) => error(
                format!("Url parameter {} error: {}", param, e),
                StatusCode::BAD_REQUEST,
            ),
            ResponseError::CreateIndex(err) => error(
                format!("Impossible to create index; {}", err),
                StatusCode::BAD_REQUEST,
            ),
            ResponseError::Maintenance => error(
                String::from("Server is in maintenance, please try again later"),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ErrorMessage {
    message: String,
}

fn error(message: String, status: StatusCode) -> Response {
    let message = ErrorMessage { message };
    tide::response::json(message)
        .with_status(status)
        .into_response()
}
