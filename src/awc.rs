use awc::{
    error::{PayloadError, SendRequestError},
    Client as AwcClient,
};
use http::{Error as HttpError, Request};

use async_trait::async_trait;
use thiserror::Error as ThisError;

use crate::{
    client::{HttpClient, HttpResponse},
    RequestError,
};

#[derive(Debug, ThisError)]
pub enum AwcClientError {
    #[error(transparent)]
    PayloadError(#[from] PayloadError),
    #[error(transparent)]
    SendRequestError(#[from] SendRequestError),
    #[error(transparent)]
    HttpError(#[from] HttpError),
}

impl Into<RequestError<AwcClient>> for AwcClientError {
    fn into(self) -> RequestError<AwcClient> {
        RequestError::HttpClientError(self)
    }
}

#[async_trait(?Send)]
impl HttpClient for AwcClient {
    type Err = AwcClientError;

    async fn request(&self, request: Request<String>) -> Result<HttpResponse, Self::Err> {
        let mut client_request = self.request(request.method().to_owned(), request.uri());

        for header_pair in request.headers().iter() {
            client_request = client_request.insert_header(header_pair);
        }

        let mut client_response = client_request.send_body(request.body().to_owned()).await?;

        Ok(HttpResponse {
            bytes: Vec::from(client_response.body().await?),
            status: client_response.status(),
        })
    }
}
