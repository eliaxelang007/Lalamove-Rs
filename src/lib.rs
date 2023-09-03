#![feature(generic_const_exprs)]

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    num::ParseIntError,
    str::FromStr,
};

use cfg_if::cfg_if;
use thiserror::Error as ThisError;

use http::Uri;
use serde::{Deserialize, Serialize};

use serde_with::{serde_as, DisplayFromStr};

use phonenumber::PhoneNumber;
use rusty_money::{iso::Currency, Money};

mod markets;

pub use markets::{
    Country, Dimensions, InvalidPhilippineLanguage, Kilograms, Language, Market, MarketInfo,
    Meters, PhilippineLanguages, PhilippineMarket, PhilippineRegions, Region, RegionError,
    RegionInfo, Service, ServiceType, SpecialRequest, SpecialRequestType,
};

cfg_if! {
    if #[cfg(feature = "_client")]
    {
        mod client;
        pub use client::{Config, ConfigError, Lalamove, QuoteError, RequestError};
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum DeliveryStatus {
    AssigningDriver,
    Ongoing,
    PickedUp,
    Completed,
    Canceled,
    Rejected,
    Expired,
}

#[derive(Debug, ThisError)]
pub enum InvalidDeliveryStatus {
    #[error("Couldn't find a corresponding delivery status for the string.")]
    NoDeliveryStatusFound,
}

impl FromStr for DeliveryStatus {
    type Err = InvalidDeliveryStatus;

    fn from_str(delivery_status: &str) -> Result<Self, Self::Err> {
        use DeliveryStatus as DS;

        let delivery_status = delivery_status.to_uppercase();

        Ok(match &*delivery_status {
            "ASSIGNING_DRIVER" => DS::AssigningDriver,
            "ON_GOING" => DS::Ongoing,
            "PICKED_UP" => DS::PickedUp,
            "COMPLETED" => DS::Completed,
            "CANCELED" => DS::Canceled,
            "REJECTED" => DS::Rejected,
            "EXPIRED" => DS::Expired,
            _ => return Err(InvalidDeliveryStatus::NoDeliveryStatusFound),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeliveryId(u64);

impl FromStr for DeliveryId {
    type Err = ParseIntError;

    fn from_str(delivery_id: &str) -> Result<Self, Self::Err> {
        Ok(DeliveryId(delivery_id.parse()?))
    }
}

impl Display for DeliveryId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DriverId(u64);

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct DeliveryRequest<const RECIPIENT_STOP_COUNT: usize>
where
    Assert<{ valid_recipient_stop_count(RECIPIENT_STOP_COUNT) }>: IsTrue,
{
    pub quoted: QuotedRequest<RECIPIENT_STOP_COUNT>,
    pub sender: PersonInfo,
    #[serde_as(as = "[_; RECIPIENT_STOP_COUNT]")]
    pub recipients_info: [PersonInfo; RECIPIENT_STOP_COUNT],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonInfo {
    pub name: String,
    pub phone_number: PhoneNumber,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct QuotedRequest<const RECIPIENT_STOP_COUNT: usize>
where
    Assert<{ valid_recipient_stop_count(RECIPIENT_STOP_COUNT) }>: IsTrue,
{
    quotation_id: QuotationId,
    pick_up_stop_id: StopId,
    #[serde_as(as = "[_; RECIPIENT_STOP_COUNT]")]
    stop_ids: [StopId; RECIPIENT_STOP_COUNT],
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationRequest<const RECIPIENT_STOP_COUNT: usize>
where
    Assert<{ valid_recipient_stop_count(RECIPIENT_STOP_COUNT) }>: IsTrue,
{
    pub service: ServiceType,
    pub pick_up_location: Location,
    #[serde_as(as = "[_; RECIPIENT_STOP_COUNT]")]
    pub stops: [Location; RECIPIENT_STOP_COUNT],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub address: String,
}

#[serde_as]
#[derive(Debug, Serialize)]
pub struct Quote {
    pub distance: Meters,
    #[serde_as(as = "DisplayFromStr")]
    pub price: Money<'static, Currency>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuotationId(u64);

#[derive(Debug, Serialize, Deserialize)]
pub struct StopId(u64);

impl Display for QuotationId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        write!(formatter, "{}", self.0)
    }
}

impl Display for StopId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for QuotationId {
    type Err = ParseIntError;

    fn from_str(quotation_id: &str) -> Result<Self, Self::Err> {
        Ok(QuotationId(quotation_id.parse()?))
    }
}

impl FromStr for StopId {
    type Err = ParseIntError;

    fn from_str(stop_id: &str) -> Result<Self, Self::Err> {
        Ok(StopId(stop_id.parse()?))
    }
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct Delivery {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename(deserialize = "orderId"))]
    pub id: DeliveryId,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename(deserialize = "shareLink"))]
    pub share_link: Uri,
}

pub const fn valid_recipient_stop_count(stop_count: usize) -> bool {
    const MAX_STOPS: usize = 15;
    const MIN_STOPS: usize = 1;

    stop_count >= MIN_STOPS && stop_count <= MAX_STOPS
}

#[derive(Debug)]
pub struct Assert<const CONDITION: bool> {}
pub trait IsTrue {}
impl IsTrue for Assert<true> {}
