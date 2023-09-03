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

#[cfg(test)]
mod tests {
    #[actix_rt::test]
    async fn main() {
        use crate::{
            Config, DeliveryRequest, Lalamove, Location, PersonInfo, PhilippineLanguages,
            PhilippineMarket, QuotationRequest,
        };
        use awc::Client;
        use dotenvy_macro::dotenv;
        use phonenumber::parse;

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
pub enum AwcClientError {
    #[error(transparent)]
    PayloadError(#[from] PayloadError),
    #[error(transparent)]
    SendRequestError(#[from] SendRequestError),
    #[error(transparent)]
    HttpError(#[from] HttpError),
}

impl From<AwcClientError> for RequestError<AwcClient> {
    fn from(value: AwcClientError) -> Self {
        RequestError::HttpClientError(value)
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
