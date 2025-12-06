//! src/meetup/scrape.rs
//!
//! handles scraping data from the meetup website
//! WARN: The sequence for fetching the meetup data is specific
//! to the HTML and JSON data scraped from meetup.com. If they
//! change the structure, this will no longer work as expected.
#![allow(unused)]

use chrono::{DateTime, FixedOffset};
use scraper::{Html, Selector};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, from_value};
use std::collections::{BTreeMap, HashMap};

use crate::meetup::model::{JSONEvent, JSONMember, JSONPhotoInfo, JSONVenue};

const MEETUP_START_URL: &str = "https://meetup.com/";
const MEETUP_END_URL: &str = "/events/?type=upcoming";

// TODO: remove this
const WATCHED_GROUPS: [&str; 2] = ["gwinnett-hangouts", "roswell-and-alpharetta-20s-30s"];

/// single event struct containing all relevant event data
/// idk about this... very messy...
/// this is a future-me problem...
#[derive(Clone)]
pub struct MeetupEvent {
    event: JSONEvent,
    venue: Option<JSONVenue>,
    members: BTreeMap<String, JSONMember>,
    photos: BTreeMap<String, JSONPhotoInfo>,
}

impl MeetupEvent {
    fn from(event: JSONEvent) -> Self {
        Self {
            event,
            venue: None,
            members: BTreeMap::new(),
            photos: BTreeMap::new(),
        }
    }
    fn set_venue(&mut self, v: Option<JSONVenue>) {
        self.venue = v;
    }
    fn add_member(&mut self, m: JSONMember) {
        self.members.insert(m.id.to_string(), m);
    }
    fn add_photo(&mut self, p: JSONPhotoInfo) {
        self.photos.insert(p.id.to_string(), p);
    }
    pub fn get_event(&self) -> &JSONEvent {
        &self.event
    }
    pub fn get_member(&self, id: &str) -> Option<&JSONMember> {
        self.members.get(id)
    }
    pub fn get_photoinfo(&self, id: &str) -> Option<&JSONPhotoInfo> {
        self.photos.get(id)
    }
    pub fn get_venue(&self, id: &str) -> Option<&JSONVenue> {
        self.venue.as_ref()
    }
}

/// contains all relevant event(s) data from a meetup group
pub struct MeetupGroupData {
    events: Vec<JSONEvent>,
    members: BTreeMap<String, JSONMember>,
    photos: BTreeMap<String, JSONPhotoInfo>,
    venues: BTreeMap<String, JSONVenue>,
}

impl MeetupGroupData {
    fn from(json: Map<String, Value>) -> Self {
        let mut meetup_data = Self {
            events: extract_sorted_events(&json, "Event:"),
            members: extract_fields(&json, "Member:"),
            photos: extract_fields(&json, "PhotoInfo:"),
            venues: extract_fields(&json, "Venue:"),
        };
        meetup_data
    }

    /// fetches all `MeetupEvents`
    pub fn get_events(&self) -> Vec<MeetupEvent> {
        self.events
            .iter()
            .map(|e| self.build_event(e.to_owned()))
            .collect()
    }
    /// fetch a reference to stored meetup member
    fn get_member(&self, id: &str) -> Option<&JSONMember> {
        self.members.get(id)
    }
    /// fetch a reference to stored meetup photoinfo
    fn get_photoinfo(&self, id: &str) -> Option<&JSONPhotoInfo> {
        self.photos.get(id)
    }
    /// fetch a reference to stored meetup venue
    fn get_venue(&self, id: &str) -> Option<&JSONVenue> {
        self.venues.get(id)
    }
    /// builds a `MeetupEvent` from collective `MeetupGroupData`
    fn build_event(&self, event: JSONEvent) -> MeetupEvent {
        let mut meetup_event = MeetupEvent::from(event.clone());
        meetup_event.set_venue(self.get_venue(&event.venue).cloned());
        let creator_member = self.get_member(&event.creatorMember);
        if let Some(c_m) = creator_member {
            let creator_photo = self.get_photoinfo(&c_m.memberPhoto);
            if let Some(c_p) = creator_photo {
                meetup_event.add_photo(c_p.clone());
            }
            meetup_event.add_member(c_m.clone());
        }
        if let Some(event_photo) = self.get_photoinfo(&event.featuredEventPhoto) {
            meetup_event.add_photo(event_photo.clone());
        }
        meetup_event
    }
}

/// fetches JSON from a 'meetup.com' group, turns it into a
/// rust-friendly data format (`MeetupGroupData`)
pub fn get_meetup_group_data(
    group_name: &str,
) -> Result<MeetupGroupData, Box<dyn std::error::Error>> {
    let json = fetch_json(group_name)?;
    let group_json = isolate_props(&json).unwrap();
    let meetup_group_data = MeetupGroupData::from(group_json);
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

/// Extracts JSON `Value`s whos keys match a partial string.
/// Used for dealing with ridiculously named JSON fields.
fn extract_fields<T: DeserializeOwned>(
    props: &Map<String, Value>,
    partial: &str,
) -> BTreeMap<String, T> {
    props
        .iter()
        .filter_map(|(k, v)| match k.find(partial) {
            Some(_) => Some((
                k.strip_prefix(partial).unwrap().to_owned(),
                from_value::<T>(v.to_owned()).expect("Could not convert `Value` to type `T`"),
            )),
            _ => None,
        })
        .collect::<BTreeMap<String, T>>()
}

/// Extracts JSON `Event`s and sorts them by earliest date
fn extract_sorted_events(props: &Map<String, Value>, partial: &str) -> Vec<JSONEvent> {
    let mut events = extract_fields::<JSONEvent>(props, partial)
        .iter()
        .map(|(_, v)| v.clone())
        .collect::<Vec<JSONEvent>>();
    // sort meetup events by date (earliest first)
    events.sort_by(|a, b| a.dateTime.cmp(&b.dateTime));
    events
}

/// fetches the JSON data containing meetup events for a particular group
/// given the URL for that groups upcoming events page
/// NOTE: fetches (up to) the next 30 upcoming meetup events and
/// associated data
fn fetch_json(group_name: &str) -> Result<String, Box<dyn std::error::Error>> {
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
