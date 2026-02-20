//! src/meetup/scrape.rs
//!
//! handles scraping data from the meetup website
//! WARN: The sequence for fetching the meetup data is specific
//! to the HTML and JSON data scraped from meetup.com. If they
//! change the structure, this will no longer work as expected.
//#![allow(unused)]

use reqwest::StatusCode;
use scraper::{Html, Selector};
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::Error;
use crate::meetup::model::MeetupEventBuilder;

/// builds the url to fetch meetup data from directly
fn build_fetch_url(group_name: &str, build_id: &str) -> String {
    format!(
        "https://www.meetup.com/_next/data/{}/en-US/{}/events.json",
        build_id, group_name
    )
}

/// builds the url to a groups upcoming events page given the name of the group
fn build_url(group_name: &str) -> String {
    format!("https://meetup.com/{}/events/?type=upcoming", group_name)
}

/// fetches JSON from a 'meetup.com' group, turns it into a
/// rust-friendly data format (`MeetupGroupData`)
pub fn get_meetup_group_data(
    group_name: &str,
    build_id: &str,
) -> Result<MeetupEventBuilder, Error> {
    let json = fetch_json(group_name, build_id)?;
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
        //.get("props")?
        .get("pageProps")?
        .get("__APOLLO_STATE__")?;
    Some(props.as_object()?.to_owned())
}

/// pulls a build_id used in a link to more directly fetch meetup.com events data from
pub fn get_build_id() -> Result<String, Error> {
    let url = "https://www.meetup.com";
    let response = reqwest::blocking::get(url)?;
    if response.status() != StatusCode::OK {
        println!(
            "[FETCH] Response status from `{}`: {}",
            url,
            response.status()
        );
        return Err("Error fetching.".into());
    }
    let document = Html::parse_document(&response.text()?);
    let selector = &Selector::parse(r#"meta[name="X-Build-Version"]"#).unwrap();
    let tags: Vec<Option<&str>> = document
        .select(selector)
        .map(|t| t.value().attr("content"))
        .collect();
    let Some(build_id) = &tags[0] else {
        return Err("Could not get build id".into());
    };
    Ok(build_id.to_string())
}

/// fetches the JSON data containing meetup events for a particular group
/// given the URL for that groups upcoming events page
/// NOTE: fetches (up to) the next 30 upcoming meetup events and
/// associated data
fn fetch_json(group_name: &str, build_id: &str) -> Result<String, Error> {
    let url = build_fetch_url(group_name, build_id);
    let response = reqwest::blocking::get(&url)?;
    if response.status() != StatusCode::OK {
        println!(
            "[FETCH] Response status from `{}`: {}",
            url,
            response.status()
        );
        return Err("Error fetching.".into());
    }

    let json = &response.text()?;

    /*
        // TODO: If parsing fails, send a message to discord guilds? More importantly, send to logs
        //
        // select all scripts containing json
        let selector = &Selector::parse(r#"script[type="application/json"]"#).unwrap();
        let scripts: Vec<String> = document.select(selector).map(|s| s.html()).collect();
        // there should only be one script in the vec
        let script = scripts[0].clone();
        // isolate the json data from the script. This contains the
        // meetup data we need
        let json = strip_outer_html(script);
    */
    Ok(json.to_string())
}

/// removes outer html tags (assumes no inner html tags)
fn strip_outer_html(html: String) -> String {
    html.split(">").collect::<Vec<&str>>()[1]
        .split("<")
        .collect::<Vec<&str>>()[0]
        .to_string()
}
