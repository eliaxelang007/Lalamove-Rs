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

        let (quoted_request, _) = lalamove
            .quote(QuotationRequest {
                pick_up_location: Location {
                    latitude: 14.535372967557564,
                    longitude: 120.98197538196277,
                    address: "SM Mall of Asia, Seaside Boulevard, 123, Pasay, Metro Manila".to_owned(),
                },
                service: market_info.regions[0].services[0].service.clone(),
                stops: [Location {
                    latitude: 14.586164229973143,
                    longitude: 121.05665251264826,
                    address: "SM Megamall, Doña Julia Vargas Avenue, Ortigas Center, Mandaluyong, Metro Manila".to_string(),
                }],
            })
            .await
            .unwrap();

        let delivery = lalamove
            .place_order(DeliveryRequest {
                quoted: quoted_request,
                sender: PersonInfo {
                    name: "Alice".to_string(),
                    phone_number: parse(None, "+639000001024").unwrap(),
                },
                recipients_info: [PersonInfo {
                    name: "Bob".to_string(),
                    phone_number: parse(None, "+639000000512").unwrap(),
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
