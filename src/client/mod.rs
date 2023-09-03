use std::{
    error::Error,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    iter::{once, zip},
    str::FromStr,
    string::FromUtf8Error,
    time::{SystemTime, UNIX_EPOCH},
};

use mime::APPLICATION_JSON;

use serde::{de::DeserializeOwned, ser::Serialize as Serializable, Deserialize, Serialize};
use serde_json::{
    error::{Category as DeJsonErrorCategory, Error as SerdeJsonError},
    from_str, from_value, json, to_value, Value,
};
use serde_with::{serde_as, DisplayFromStr};

use hex::encode;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Method, Request, StatusCode,
};

use thiserror::Error as ThisError;

use phonenumber::PhoneNumber;
use rusty_money::{iso, Money, MoneyError};

use crate::{
    markets::Language, valid_recipient_stop_count, Assert, Delivery, DeliveryId,
    DeliveryRequest, DeliveryStatus, IsTrue, Location, Market, MarketInfo, Meters, QuotationId,
    QuotationRequest, Quote, QuotedRequest, ServiceType, StopId,
};

use async_trait::async_trait;
use cfg_if::cfg_if;

pub struct HttpResponse {
    pub status: StatusCode,
    pub bytes: Vec<u8>,
}

cfg_if! {
    if #[cfg(feature = "awc")] {
        mod awc;

        #[async_trait(?Send)]
        pub trait HttpClient: Default {
            type Err: Error + Into<RequestError<Self>>;
            async fn request(&self, request: Request<String>) -> Result<HttpResponse, Self::Err>;
        }
    } else if #[cfg(feature = "reqwest")] {
        mod reqwest;

        #[async_trait]
        pub trait HttpClient: Default {
            type Err: Error + Debug + Into<RequestError<Self>>;
            async fn request(&self, request: Request<String>) -> Result<HttpResponse, Self::Err>;
        }
    } else {

    }
}

#[derive(Clone)]
pub struct Lalamove<M: Market, C: HttpClient>
where
    <<M as Market>::Languages as FromStr>::Err: Error,
{
    client: C,
    config: Config<M>,
}

impl<M: Market, C: HttpClient> Lalamove<M, C>
where
    <<M as Market>::Languages as FromStr>::Err: Error,
{
    pub fn new(config: Config<M>) -> Self {
        Lalamove {
            config,
            client: C::default(),
        }
    }
}

#[derive(ThisError)]
pub enum QuoteError<C: HttpClient> {
    #[error(transparent)]
    RequestError(#[from] RequestError<C>),
    #[error("Couldn't find a currency that matched the one in the price breakdown.")]
    CurrencyNotFound,
    #[error(transparent)]
    MoneyError(#[from] MoneyError),
}

impl<C: HttpClient> Debug for QuoteError<C>
where
    C::Err: Error,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::RequestError(e) => write!(f, "RequestError({:?})", e),
            Self::MoneyError(e) => write!(f, "MoneyError({:?})", e),
            Self::CurrencyNotFound => write!(f, "CurrencyNotFound"),
        }
    }
}

impl<M: Market, C: HttpClient> Lalamove<M, C>
where
    <<M as Market>::Languages as FromStr>::Err: Error,
{
    pub async fn market_info(&self) -> Result<MarketInfo, RequestError<C>> {
        self.make_request::<MarketInfo>(ApiPaths::Cities, Method::GET, None::<()>)
            .await
    }

    pub async fn quote<const RECIPIENT_STOP_COUNT: usize>(
        &self,
        request: QuotationRequest<RECIPIENT_STOP_COUNT>,
    ) -> Result<(QuotedRequest<RECIPIENT_STOP_COUNT>, Quote), QuoteError<C>>
    where
        Assert<{ valid_recipient_stop_count(RECIPIENT_STOP_COUNT) }>: IsTrue,
        [Location; RECIPIENT_STOP_COUNT + 1]: Sized,
    {
        let request_clone = request.clone();

        let api_request = ApiQuotationRequest {
            service_type: request_clone.service,
            stops: {
                let locations = once(request_clone.pick_up_location)
                        .chain(request_clone.stops)
                        .map(|location| location.into())
                        .collect::<Vec<_>>()
                        .try_into()
                        .expect("This shouldn't fail because the stops array's size is RECIPIENT_STOP_COUNT + 1.");

                locations
            },
            language: self.config.language.language_code().to_owned(),
        };

        let response = self
            .make_request::<ApiQuote<RECIPIENT_STOP_COUNT>>(
                ApiPaths::Quotations,
                Method::POST,
                Some(api_request),
            )
            .await?;

        let mut stops = response.stops.into_iter().map(|api_stop| api_stop.stop_id);
        let pick_up_stop_id = stops
            .next()
            .expect("There should have been a Stop ID for the pick up location!");
        let stop_ids = stops
            .collect::<Vec<_>>()
            .try_into()
            .expect("There should be enough Stop IDs for the drop off locations!");

        return Ok((
            QuotedRequest {
                quotation_id: response.quotation_id,
                pick_up_stop_id,
                stop_ids,
            },
            Quote {
                distance: response.distance,
                price: {
                    let currency = iso::find(&*response.price_breakdown.currency)
                        .ok_or(QuoteError::CurrencyNotFound)?;

                    Money::from_str(&*response.price_breakdown.total, currency)?
                },
            },
        ));

        #[serde_as]
        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ApiQuote<const RECIPIENT_STOP_COUNT: usize>
        where
            Assert<{ valid_recipient_stop_count(RECIPIENT_STOP_COUNT) }>: IsTrue,
            [Location; RECIPIENT_STOP_COUNT + 1]: Sized,
        {
            distance: Meters,
            price_breakdown: ApiPriceBreakdown,
            #[serde_as(as = "DisplayFromStr")]
            quotation_id: QuotationId,
            #[serde_as(as = "[_; RECIPIENT_STOP_COUNT + 1]")]
            stops: [ApiStopId; RECIPIENT_STOP_COUNT + 1],
        }

        #[serde_as]
        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ApiStopId {
            #[serde_as(as = "DisplayFromStr")]
            stop_id: StopId,
        }

        #[serde_as]
        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ApiPriceBreakdown {
            total: String,
            currency: String,
        }

        #[serde_as]
        #[derive(Serialize, Debug)]
        struct ApiCoordinates {
            #[serde_as(as = "DisplayFromStr")]
            lat: f64,
            #[serde_as(as = "DisplayFromStr")]
            lng: f64,
        }

        #[derive(Serialize, Debug)]
        struct ApiLocation {
            coordinates: ApiCoordinates,
            address: String,
        }

        impl From<Location> for ApiLocation {
            fn from(location: Location) -> Self {
                ApiLocation {
                    coordinates: ApiCoordinates {
                        lat: location.latitude,
                        lng: location.longitude,
                    },
                    address: location.address,
                }
            }
        }

        #[serde_as]
        #[derive(Serialize, Debug)]
        struct ApiQuotationRequest<const RECIPIENT_STOP_COUNT: usize>
        where
            Assert<{ valid_recipient_stop_count(RECIPIENT_STOP_COUNT) }>: IsTrue,
            [Location; RECIPIENT_STOP_COUNT + 1]: Sized,
        {
            #[serde(rename(serialize = "serviceType"))]
            service_type: ServiceType,
            #[serde_as(as = "[_; RECIPIENT_STOP_COUNT + 1]")]
            stops: [ApiLocation; RECIPIENT_STOP_COUNT + 1],
            language: String,
        }
    }

    pub async fn place_order<const RECIPIENT_STOP_COUNT: usize>(
        &self,
        request: DeliveryRequest<RECIPIENT_STOP_COUNT>,
    ) -> Result<Delivery, RequestError<C>>
    where
        Assert<{ valid_recipient_stop_count(RECIPIENT_STOP_COUNT) }>: IsTrue,
    {
        let request = ApiDeliveryRequest {
            quotation_id: request.quoted.quotation_id,
            sender: ApiStopInfo {
                stop_id: request.quoted.pick_up_stop_id,
                name: request.sender.name,
                phone: request.sender.phone_number,
            },
            recipients: zip(request.recipients_info, request.quoted.stop_ids)
                .map(|(recipient_info, stop_id)| ApiStopInfo {
                    stop_id,
                    name: recipient_info.name,
                    phone: recipient_info.phone_number,
                })
                .collect::<Vec<_>>()
                .try_into()
                .expect("There should be enough Stop IDs for the drop off locations!"),
        };

        return self
            .make_request(ApiPaths::Orders, Method::POST, Some(request))
            .await;

        #[serde_as]
        #[derive(Serialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ApiDeliveryRequest<const RECIPIENT_STOP_COUNT: usize>
        where
            Assert<{ valid_recipient_stop_count(RECIPIENT_STOP_COUNT) }>: IsTrue,
        {
            #[serde_as(as = "DisplayFromStr")]
            quotation_id: QuotationId,
            sender: ApiStopInfo,
            #[serde_as(as = "[_; RECIPIENT_STOP_COUNT]")]
            recipients: [ApiStopInfo; RECIPIENT_STOP_COUNT],
        }

        #[serde_as]
        #[derive(Serialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ApiStopInfo {
            #[serde_as(as = "DisplayFromStr")]
            stop_id: StopId,
            name: String,
            #[serde_as(as = "DisplayFromStr")]
            phone: PhoneNumber,
        }
    }

    pub async fn delivery_status(
        &self,
        delivery: DeliveryId,
    ) -> Result<DeliveryStatus, RequestError<C>> {
        return Ok(self
            .make_request::<ApiDeliveryDetails>(
                ApiPaths::Order(delivery),
                Method::GET,
                None::<()>,
            )
            .await?
            .status);

        #[serde_as]
        #[derive(Deserialize, Debug)]
        struct ApiDeliveryDetails {
            #[serde_as(as = "DisplayFromStr")]
            status: DeliveryStatus,
        }
    }

    async fn make_request<'a, T: DeserializeOwned>(
        &self,
        path: ApiPaths,
        method: Method,
        body: Option<impl Serializable>,
    ) -> Result<T, RequestError<C>> {
        let body = body.map(|body| to_value(body));
        let body = match body {
            Some(serialized) => Some(serialized?),
            None => None,
        };

        let request = self.config.build_request(path, method, body);
        let response = match self.client.request(request).await {
            Ok(response) => response,
            Err(error) => return Err(error.into()),
        };

        let response_string = String::from_utf8(response.bytes)?;
        let response_json = from_str::<Value>(&response_string);

        return match response_json {
            Ok(response) => {
                use RequestError::NoData;
                use Value::*;
                match response {
                    Object(mut map) => {
                        let data = map.get_mut("data");

                        match data {
                            Some(data) => Ok(from_value::<T>(data.take())?),
                            None => Err(if map.contains_key("errors") {
                                RequestError::ApiError(ApiError::Json(Object(map)))
                            } else {
                                NoData
                            }),
                        }
                    }
                    _ => Err(NoData),
                }
            }
            Err(error) => {
                use DeJsonErrorCategory::*;

                Err(match error.classify() {
                    Syntax => RequestError::ApiError(ApiError::InvalidJson(response_string)),
                    _ => RequestError::SerdeJsonError(error),
                })
            }
        };
    }
}

#[derive(Debug, ThisError)]
pub enum ApiError {
    #[error("The Lalamove API responded with the non json string '{0:?}'.")]
    InvalidJson(String),
    #[error(
        "The Lalamove API responded with the json '{0:?}' which could not be deserialized."
    )]
    Json(Value),
}

#[derive(ThisError)]
pub enum RequestError<C: HttpClient>
where
    C::Err: Error,
{
    #[error(transparent)]
    HttpClientError(C::Err),
    #[error(transparent)]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error(transparent)]
    ApiError(#[from] ApiError),
    #[error(transparent)]
    SerdeJsonError(#[from] SerdeJsonError),
    #[error("The json response from Lalamove didn't have the 'data' key in it.")]
    NoData,
}

impl<C: HttpClient> Debug for RequestError<C>
where
    C::Err: Error,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::HttpClientError(e) => write!(f, "HttpClientError({:?})", e),
            Self::FromUtf8Error(e) => write!(f, "FromUtf8Error({:?})", e),
            Self::ApiError(e) => write!(f, "ApiError({:?})", e),
            Self::SerdeJsonError(e) => write!(f, "SerdeJsonError({:?})", e),
            Self::NoData => write!(f, "NoData"),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Config<M: Market>
where
    <<M as Market>::Languages as FromStr>::Err: Error,
{
    pub api_key: String,
    pub api_secret: String,
    pub language: M::Languages,
    pub environment: ApiEnvironment,
}

impl<M: Market> Config<M>
where
    <<M as Market>::Languages as FromStr>::Err: Error,
{
    pub fn new(
        api_key: String,
        api_secret: String,
        language: M::Languages,
    ) -> Result<Self, ConfigError> {
        let api_key_environment = ApiEnvironment::from_str(&api_key)?;
        let api_secret_environment = ApiEnvironment::from_str(&api_secret)?;

        if api_key_environment != api_secret_environment {
            return Err(ConfigError::IncompatibleKeyAndSecret);
        }

        Ok(Config {
            api_key,
            api_secret,
            language,
            environment: api_key_environment,
        })
    }

    fn build_request(
        &self,
        path: ApiPaths,
        method: Method,
        body: Option<Value>,
    ) -> Request<String> {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get the current system time!")
            .as_millis();

        let body = body.map(|value| json!({ "data": value }));

        let body_str = body
            .as_ref()
            .map(|value| value.to_string())
            .unwrap_or("".to_string());

        let path = path.to_string();

        let raw_signature = format!("{time}\r\n{method}\r\n{path}\r\n\r\n{body_str}");

        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())
            .expect("Failed to interpret the API SECRET as bytes!");
        mac.update(raw_signature.as_bytes());

        let signature = encode(mac.finalize().into_bytes());

        let api_key = &self.api_key;
        let application_json = APPLICATION_JSON.to_string();

        Request::builder()
            .method(method)
            .uri(self.environment.base_url().to_string() + &path)
            .header(ACCEPT, application_json.clone())
            .header(CONTENT_TYPE, application_json)
            .header(AUTHORIZATION, format!("hmac {api_key}:{time}:{signature}"))
            .header("Market", M::country().country_code())
            .body(body_str)
            .expect("This should have been a valid request.")
    }
}

#[derive(Debug, ThisError)]
pub enum ConfigError {
    #[error("The API key and the API secret were not from the same environment.")]
    IncompatibleKeyAndSecret,
    #[error(transparent)]
    ApiEnvironmentError(#[from] ApiEnvironmentError),
}

#[derive(Debug, Serialize)]
enum ApiPaths {
    Cities,
    Quotations,
    Orders,
    Order(DeliveryId),
}

impl ApiPaths {
    fn path(&self) -> String {
        use ApiPaths::*;

        (match self {
            Cities => "/v3/cities",
            Quotations => "/v3/quotations",
            Orders => "/v3/orders",
            Order(id) => return format!("/v3/orders/{id}"),
        })
        .to_string()
    }
}

impl Display for ApiPaths {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        write!(formatter, "{}", self.path())
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Clone)]
pub enum ApiEnvironment {
    Sandbox,
    Production,
}

impl ApiEnvironment {
    const fn base_url(&self) -> &'static str {
        use ApiEnvironment::*;

        match self {
            Sandbox => "https://rest.sandbox.lalamove.com",
            Production => "https://rest.lalamove.com",
        }
    }
}

impl FromStr for ApiEnvironment {
    type Err = ApiEnvironmentError;

    fn from_str(api_key_or_api_secret: &str) -> Result<Self, Self::Err> {
        let environment = api_key_or_api_secret.chars().skip(3).collect::<String>();

        use ApiEnvironment::*;
        use ApiEnvironmentError::*;

        if environment.starts_with("test") {
            Ok(Sandbox)
        } else if environment.starts_with("prod") {
            Ok(Production)
        } else {
            Err(InvalidApiKeyOrApiSecret)
        }
    }
}

#[derive(Debug, ThisError)]
pub enum ApiEnvironmentError {
    #[error("The environment of the API key or API secret couldn't be parsed correctly.")]
    InvalidApiKeyOrApiSecret,
}
