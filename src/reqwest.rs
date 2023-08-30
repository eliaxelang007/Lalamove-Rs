use http::{Error as HttpError, Request};
use reqwest::{Client as ReqwestClient, Error as ReqwestError};

use async_trait::async_trait;
use thiserror::Error as ThisError;

use crate::{
    client::{HttpClient, HttpResponse},
    RequestError,
};

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn main() {
        use crate::{
            Config, DeliveryRequest, Lalamove, Location, PersonInfo, PhilippineLanguages,
            PhilippineMarket, QuotationRequest,
        };
        use dotenvy_macro::dotenv;
        use phonenumber::parse;
        use reqwest::Client;

        let lalamove = Lalamove::<PhilippineMarket, Client>::new(
            Config::new(
                dotenv!("LALAMOVE_API_KEY").to_string(),
                dotenv!("LALAMOVE_API_SECRET").to_string(),
                PhilippineLanguages::English,
            )
            .unwrap(),
        );

        let market_info = lalamove.market_info().await.unwrap();

        // Goodluck Lalamove driver.
        let (quoted_request, _) = lalamove
            .quote(QuotationRequest {
                pick_up_location: Location {
                    latitude: 48.85846183491826,
                    longitude: 2.294438381392602,
                    address: "Eiffel Tower, Avenue Anatole France, Paris, France".to_owned(),
                },
                service: market_info.regions[0].services[0].service.clone(),
                stops: [Location {
                    latitude: 41.90258651478627,
                    longitude: 12.453863630073503,
                    address: "St. Peter's Basilica, Piazza San Pietro, Vatican City".to_string(),
                }],
            })
            .await
            .unwrap();

        let delivery = lalamove
            .place_order(DeliveryRequest {
                quoted: quoted_request,
                sender: PersonInfo {
                    name: "Alice".to_string(),
                    phone_number: parse(None, "1024").unwrap(),
                },
                recipients_info: [PersonInfo {
                    name: "Bob".to_string(),
                    phone_number: parse(None, "512").unwrap(),
                }],
            })
            .await
            .unwrap();

        println!("{delivery:?}");
    }
}

#[derive(Debug, ThisError)]
pub enum ReqwestClientError {
    #[error(transparent)]
    ReqwestError(#[from] ReqwestError),
    #[error(transparent)]
    HttpError(#[from] HttpError),
}

impl Into<RequestError<ReqwestClient>> for ReqwestClientError {
    fn into(self) -> RequestError<ReqwestClient> {
        RequestError::HttpClientError(self)
    }
}

#[async_trait(?Send)]
impl HttpClient for ReqwestClient {
    type Err = ReqwestClientError;

    async fn request(&self, request: Request<String>) -> Result<HttpResponse, Self::Err> {
        let mut client_request =
            self.request(request.method().to_owned(), request.uri().to_string());

        for (header_name, header_value) in request.headers().iter() {
            client_request = client_request.header(header_name, header_value)
        }

        let response = client_request
            .body(request.body().to_owned())
            .send()
            .await?;

        Ok(HttpResponse {
            status: response.status(),
            bytes: Vec::from(response.bytes().await?),
        })
    }
}
