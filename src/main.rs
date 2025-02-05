#![allow(dead_code)]
#![allow(unused_variables)]
extern crate core;

mod ptouch;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Error;
use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
struct ApiResponse {
    count: u32,
    next: Option<String>,
    previous: Option<String>,
    results: Vec<Cable>,
}

#[derive(Deserialize)]
struct Cable {
    label: String,
    id: u32,
}

fn print_label(text: &str) {
    println!("I would print: {}", text);
}

fn netbox() -> Result<(), Error> {
    let netbox_token = format!(
        "Token {}",
        env::var("NETBOX_TOKEN").expect("NETBOX_TOKEN required!")
    );
    let mut headers = HeaderMap::new();
    headers.append(
        "Authorization",
        HeaderValue::from_str(&netbox_token).unwrap(),
    );
    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()?;
    let netbox_url = format!("{}/api/dcim/cables/",env::var("NETBOX_URL").expect("NETBOX_URL required"));
    let mut response: ApiResponse = client
        .get(netbox_url)
        .send()?
        .json()?;
    while response.next.is_some() {
        for cable in response.results {
            print_label(&cable.label);
        }
        response = client.get(response.next.unwrap()).send()?.json()?;
    }
    Ok(())
}

fn main() {
    let printer = ptouch::Printer::init();
    println!("Hello {:?}", printer.model);
    println!("Status {:?}", printer.get_status());
    printer.auto_cut(true);
    printer.advanced_settings(false, true);
    printer.print("Hello");
    println!("Status {:?}", printer.get_status());
    println!("Status {:?}", printer.get_status());
    println!("Status {:?}", printer.get_status());
}
