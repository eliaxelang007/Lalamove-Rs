
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
```

## Documentation

Sadly, there's no documentation for this project yet.
It's still a work in progress!

## Additional information

As of now, this library only supports the Philippine market (because that what I needed for my use case), but I plan to flesh this package out in the future. Pull requests are welcome! I'm still a novice rust programmer so I'll need all the help I can get building this library.

## Changelog
0.1.3
- Separated [latitude] and [longitude] fields from [Location] into a new type called [Coordinates].

0.1.2
- Implemented [Debug] manually for some error types to fix some trait bound errors.
  
0.1.1
- Made the [Lalamove] client cloneable.