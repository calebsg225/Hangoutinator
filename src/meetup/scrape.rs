//! src/meetup/scrape.rs
//!
//! handles scraping data from the meetup website
#![allow(unused)]

use std::collections::HashMap;

use scraper::{Html, Selector};
use serde_json::{Map, Value, from_str};

use crate::meetup::structure::{Event, FieldType, Member, PhotoInfo, Venue};

const MEETUP_START_URL: &str = "https://meetup.com/";
const MEETUP_END_URL: &str = "/events/?type=upcoming";

// TODO: remove this
const WATCHED_GROUPS: [&str; 2] = ["gwinnett-hangouts", "roswell-and-alpharetta-20s-30s"];

/// gets the props map containing all events, members, groups, venues, etc.
/// WARN: This function is specific to the meetup JSON data scraped from
/// their web page. If they change the structure, this will no longer work.
/// TODO: deal with `expect`s
fn isolate_props(group: &str, json: &str) -> Map<String, Value> {
    let json_map = serde_json::from_str::<HashMap<String, Value>>(json).unwrap();
    let props = json_map
        .get("props")
        .expect("Could not find `props` field.")
        .get("pageProps")
        .expect("Could not find `pageProps` field.")
        .get("__APOLLO_STATE__")
        .expect("Could not find `__APOLLO_STATE__` field.");
    props
        .as_object()
        .expect("The JSON `Value` found is not an object.")
        .to_owned()
}

/// Extracts JSON `Value`s whos keys match a partial string.
/// Used for dealing with ridiculously named JSON fields.
fn extract_fields(props: &Map<String, Value>, partial: &str) -> Vec<Value> {
    props
        .iter()
        .filter_map(|(k, v)| match k.find(partial) {
            Some(_) => Some(v.to_owned()),
            _ => None,
        })
        .collect::<Vec<Value>>()
}

/// fetches the JSON data containing meetup events for a particular group
/// given the URL for that groups upcoming events page
fn fetch_json(group: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = build_url(group);
    let response = reqwest::blocking::get(&url)?;
    println!(
        "Response status fetching from `{}`: {}",
        url,
        response.status()
    );

    let document = Html::parse_document(&response.text()?);

    // select all scripts containing json
    let selector = &Selector::parse(r#"script[type="application/json"]"#).unwrap();
    let scripts: Vec<String> = document.select(selector).map(|s| s.html()).collect();
    // there should only be one script in the vec
    let script = scripts[0].clone();
    // isolate the json data from the script. This contains the
    // meetup data we need
    let json = strip_outer_html(script);
    Ok(json.to_string())
}

/// builds the url to a groups upcoming events page given the name of the group
fn build_url(group: &str) -> String {
    format!("{}{}{}", MEETUP_START_URL, group, MEETUP_END_URL)
}

/// removes outer html tags (assumes no inner html tags)
/// WARN: could be problematic depending on input string
fn strip_outer_html(html: String) -> String {
    html.split(">").collect::<Vec<&str>>()[1]
        .split("<")
        .collect::<Vec<&str>>()[0]
        .to_string()
}
