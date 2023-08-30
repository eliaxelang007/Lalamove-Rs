use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use thiserror::Error as ThisError;

use serde::{
    de::{Error as DeError, Unexpected},
    Deserialize, Deserializer, Serialize,
};

use serde_with::{serde_as, DisplayFromStr};

pub trait Market
where
    <<Self as Market>::Languages as FromStr>::Err: Display,
{
    type Languages: Language + Clone;
    fn country() -> Country;
}

pub trait Language: FromStr
where
    Self::Err: Display,
{
    fn language_code(&self) -> &'static str;
}

#[derive(Debug, Serialize, Clone)]
pub struct PhilippineMarket;

impl Market for PhilippineMarket {
    type Languages = PhilippineLanguages;

    fn country() -> Country {
        Country::Philippines
    }
}

#[derive(Debug, Clone)]
pub enum PhilippineLanguages {
    English,
}

impl Language for PhilippineLanguages {
    fn language_code(&self) -> &'static str {
        use PhilippineLanguages::*;

        match self {
            English => "en_PH",
        }
    }
}

#[derive(Debug, ThisError)]
pub enum InvalidPhilippineLanguage {
    #[error("Couldn't find a corresponding language for the language code.")]
    NoLanguageCodeFound,
}

impl FromStr for PhilippineLanguages {
    type Err = InvalidPhilippineLanguage;

    fn from_str(language_code: &str) -> Result<Self, Self::Err> {
        use PhilippineLanguages::*;

        let language_code = language_code.to_lowercase();

        Ok(match &*language_code {
            "en_ph" => English,
            _ => return Err(InvalidPhilippineLanguage::NoLanguageCodeFound),
        })
    }
}

pub enum Country {
    Philippines,
}

impl Country {
    pub const fn country_code(&self) -> &'static str {
        use Country::*;

        match self {
            Philippines => "PH",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Region {
    Philippines(PhilippineRegions),
}

impl Display for Region {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        use PhilippineRegions::*;
        use Region::*;

        write!(
            formatter,
            "{}",
            match self {
                Philippines(region) => match region {
                    Cebu => "PH CEB",
                    Manila => "PH MNL",
                    Pampanga => "PH PAM",
                },
            }
        )
    }
}

#[derive(Debug, Clone)]
pub enum PhilippineRegions {
    Cebu,
    Manila,
    Pampanga,
}

impl FromStr for Region {
    type Err = RegionError;

    fn from_str(region: &str) -> Result<Region, RegionError> {
        use PhilippineRegions::*;
        use Region::*;

        let region = region.to_lowercase();

        Ok(Philippines(match &*region {
            "ph ceb" => Cebu,
            "ph mnl" => Manila,
            "ph pam" => Pampanga,
            _ => {
                return Err(RegionError::InvalidString);
            }
        }))
    }
}

// impl Region {
//     const fn location_code(&self) -> &'static str {
//         use Region::*;

//         match self {
//             Philippines(region) => {
//                 use PhilippineRegions::*;

//                 match region {
//                     Cebu => "PH CEB",
//                     Manila => "PH MNL",
//                     Pampanga => "PH PAM",
//                 }
//             }
//         }
//     }
// }

#[derive(Debug, ThisError)]
pub enum RegionError {
    #[error("Couldn't parse the location code of the region!")]
    InvalidString,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(transparent)]
pub struct MarketInfo {
    pub regions: Vec<RegionInfo>,
}

#[serde_as]
#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct RegionInfo {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename(deserialize = "locode"))]
    pub region: Region,
    pub services: Vec<Service>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Service {
    #[serde(rename(deserialize = "key"))]
    pub service: ServiceType,
    pub description: String,
    pub dimensions: Dimensions,
    pub load: Kilograms,
    #[serde(rename(deserialize = "specialRequests"))]
    pub special_requests: Vec<SpecialRequest>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
pub struct ServiceType(String);

impl Display for ServiceType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SpecialRequest {
    pub description: String,
    #[serde(rename(deserialize = "name"))]
    pub special_request: SpecialRequestType,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(transparent)]
pub struct SpecialRequestType(String);

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Dimensions {
    pub width: Meters,
    pub height: Meters,
    pub length: Meters,
}

#[derive(Debug, Serialize, Clone)]
pub struct Meters(pub f32);

#[derive(Debug, Serialize, Clone)]
pub struct Kilograms(pub f32);

impl<'de> Deserialize<'de> for Kilograms {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let measurement = MeasurementDeserialization::deserialize(deserializer)?;

        if measurement.unit != "kg" {
            return Err(DeError::invalid_value(
                Unexpected::Str(&*measurement.unit),
                &"kg",
            ));
        }

        Ok(Kilograms(measurement.value))
    }
}

impl<'de> Deserialize<'de> for Meters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let measurement = MeasurementDeserialization::deserialize(deserializer)?;

        if measurement.unit != "m" {
            return Err(DeError::invalid_value(
                Unexpected::Str(&*measurement.unit),
                &"m",
            ));
        }

        Ok(Meters(measurement.value))
    }
}

#[serde_as]
#[derive(Deserialize)]
struct MeasurementDeserialization {
    unit: String,
    #[serde_as(as = "DisplayFromStr")]
    value: f32,
}
