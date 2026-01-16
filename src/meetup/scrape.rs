//! src/meetup/scrape.rs
//!
//! handles scraping data from the meetup website
//! WARN: The sequence for fetching the meetup data is specific
//! to the HTML and JSON data scraped from meetup.com. If they
//! change the structure, this will no longer work as expected.
//#![allow(unused)]

use scraper::{Html, Selector};
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::Error;
use crate::meetup::model::MeetupEventBuilder;

const MEETUP_START_URL: &str = "https://meetup.com/";
const MEETUP_END_URL: &str = "/events/?type=upcoming";

/// fetches JSON from a 'meetup.com' group, turns it into a
/// rust-friendly data format (`MeetupGroupData`)
pub fn get_meetup_group_data(group_name: &str) -> Result<MeetupEventBuilder, Error> {
    let json = fetch_json(group_name)?;
    let group_json = isolate_props(&json).unwrap();
    if group_json.len() <= 1 {
        return Err("Not a real meetup group page.".into());
    };
    let meetup_group_data = MeetupEventBuilder::from(group_json);
    Ok(meetup_group_data)
}

/// gets the props map containing all events, members, venues, etc.
fn isolate_props(json: &str) -> Option<Map<String, Value>> {
    let json_map = serde_json::from_str::<HashMap<String, Value>>(json).unwrap();
    let props = json_map
        .get("props")?
        .get("pageProps")?
        .get("__APOLLO_STATE__")?;
    Some(props.as_object()?.to_owned())
}

/// fetches the JSON data containing meetup events for a particular group
/// given the URL for that groups upcoming events page
/// NOTE: fetches (up to) the next 30 upcoming meetup events and
/// associated data
fn fetch_json(group_name: &str) -> Result<String, Error> {
    let url = build_url(group_name);
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
fn build_url(group_name: &str) -> String {
    format!("{}{}{}", MEETUP_START_URL, group_name, MEETUP_END_URL)
}

/// removes outer html tags (assumes no inner html tags)
fn strip_outer_html(html: String) -> String {
    html.split(">").collect::<Vec<&str>>()[1]
        .split("<")
        .collect::<Vec<&str>>()[0]
        .to_string()
}
