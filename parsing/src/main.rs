use serde_json::{from_str, Value};
use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    fs::read_to_string,
};

trait Token: Display + Debug {}

impl Token for &'static str {}
impl Token for String {}

struct Property {
    name: String,
    value: Box<dyn Token>,
}

impl Property {
    fn new<T: Token + Clone + 'static>(name: impl Into<String>, value: T) -> Self {
        Property {
            name: name.into(),
            value: Box::new(value),
        }
    }
}

struct Variant {
    name: String,
    properties: Vec<Property>,
}

impl Variant {
    fn new(name: impl Into<String>, properties: Vec<Property>) -> Self {
        Variant {
            name: name.into(),
            properties,
        }
    }
}

struct Enum {
    name: String,
    variants: Vec<Variant>,
}

impl Display for Enum {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        writeln!(f, "enum {} {{", self.name)?;

        let variants = &self.variants;

        for variant in variants {
            writeln!(f, "   {},", variant.name)?;
        }

        writeln!(f, "}}\n")?;

        writeln!(f, "impl {} {{", self.name)?;

        let mut variant_properties = HashMap::new();

        for variant in variants {
            for property in &variant.properties {
                let property_values = variant_properties
                    .entry(&property.name)
                    .or_insert_with(|| HashMap::new());
                property_values.insert(&variant.name, &property.value);
            }
        }

        for (property_name, property_values) in variant_properties {
            writeln!(f, "   fn {property_name}(&self) -> () {{");

            writeln!(f, "       match &self");

            writeln!(f, "}}");
        }

        writeln!(f, "}}")
    }
}

impl Enum {
    fn new(name: impl Into<String>, variants: Vec<Variant>) -> Self {
        Enum {
            name: name.into(),
            variants,
        }
    }
}

fn main() {
    // let market_info_str = read_to_string("./market_info.json").unwrap();
    // let market_info_json = from_str::<Value>(&market_info_str).unwrap();

    // println!("{market_info_json:?}");

    let cardinal_directions = Enum::new(
        "CardinalDirections",
        vec![
            Variant::new("North", vec![Property::new("North", "fuck")]),
            Variant::new("South", vec![]),
            Variant::new("East", vec![]),
            Variant::new("West", vec![]),
        ],
    );

    println!("{}", cardinal_directions)
}
