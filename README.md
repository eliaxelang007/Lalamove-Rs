
## About

This is an unofficial Lalamove API library for Rust!

## Installation

* Install this package from [crates.io](https://crates.io/crates/lalamove-rs).

## Usage

Here's some sample code that shows you how to place an order.

```rust
// DISCLAIMER: This code snippet is untested.

use lalamove_rs::{
    Config, DeliveryRequest, Lalamove, Location, PersonInfo, PhilippineLanguages,
    PhilippineMarket, QuotationRequest,
};
use dotenvy_macro::dotenv;
use phonenumber::parse;
use reqwest::Client;

#[tokio::main]
async fn main() {
    let lalamove = Lalamove::<PhilippineMarket, Client>::new(
        Config::new(
            dotenv!("LALAMOVE_API_KEY").to_string(),
            dotenv!("LALAMOVE_API_SECRET").to_string(),
            PhilippineLanguages::English,
        )
        .unwrap(),
    );

    let market_info = lalamove.market_info().await.unwrap();

    // Good luck Lalamove driver :P
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
```

## Documentation

Sadly, there's no documentation for this project yet.
It's still a work in progress!

## Additional information

As of now, this library only supports the Philippine market (because that what I needed for my use case), but I plan to flesh this package out in the future. Pull requests are welcome! I'm still a novice rust programmer so I'll need all the help I can get building this library.