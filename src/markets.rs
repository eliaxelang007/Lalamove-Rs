use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use thiserror::Error as ThisError;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone)]
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
        use PhilippineLanguages as PL;

        match self {
            PL::English => "en_PH",
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
        use PhilippineLanguages as PS;

        let language_code = language_code.to_lowercase();

        Ok(match &*language_code {
            "en_ph" => PS::English,
            _ => return Err(InvalidPhilippineLanguage::NoLanguageCodeFound),
        })
    }
}

pub enum Country {
    Philippines,
}

impl Country {
    pub const fn country_code(&self) -> &'static str {
        use Country as C;

        match self {
            C::Philippines => "PH",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Region {
    Philippines(PhilippineRegions),
}

impl Display for Region {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        use PhilippineRegions as PR;
        use Region as R;

        write!(
            formatter,
            "{}",
            match self {
                R::Philippines(region) => match region {
                    PR::Cebu => "PH CEB",
                    PR::Manila => "PH MNL",
                    PR::Pampanga => "PH PAM",
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
        use PhilippineRegions as PR;
        use Region as R;

        let region = region.to_lowercase();

        Ok(R::Philippines(match &*region {
            "ph ceb" => PR::Cebu,
            "ph mnl" => PR::Manila,
            "ph pam" => PR::Pampanga,
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

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct MarketInfo {
    pub regions: Vec<RegionInfo>,
}

#[serde_as]
#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct RegionInfo {
    #[serde_as(as = "DisplayFromStr")]
    pub region: Region,
    pub services: Vec<Service>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Service {
    pub service: ServiceType,
    pub description: String,
    pub dimensions: Dimensions,
    pub load: Kilograms,
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
    pub special_request: SpecialRequestType,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(transparent)]
pub struct SpecialRequestType(String);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dimensions {
    pub width: Meters,
    pub height: Meters,
    pub length: Meters,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meters(pub f32);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Kilograms(pub f32);
