//! src/meetup/scrape.rs

use std::collections::BTreeMap;

use scraper::{Html, Selector};

// using `consts` instead of `.env` vars.
// NOTE: If watched groups are to be changed from the discord server (by admins),
// a db needs to be used.
const MEETUP_START_URL: &str = "https://meetup.com/";
const MEETUP_END_URL: &str = "/events/?type=upcoming";
const WATCHED_GROUPS: [&str; 2] = ["gwinnett-hangouts", "roswell-and-alpharetta-20s-30s"];

/// fetches all events from each group in `WATCHED_GROUPS`
fn fetch_events() {}

/// fetches the JSON data containing meetup events for a particular group
/// given the URL for that groups upcoming events page
fn fetch_page_json(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::blocking::get(url)?;
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
    let script = &scripts[0].clone();
    // isolate the json data from the script. This contains the
    // meetup event data we need
    let json = strip_outer_html(script);
    Ok(json.to_string())
}

/// removes outer html tags (assumes no inner html tags)
/// NOTE: could be problematic depending on input string
fn strip_outer_html(html: &str) -> &str {
    html.split(">").collect::<Vec<&str>>()[1]
        .split("<")
        .collect::<Vec<&str>>()[0]
}
