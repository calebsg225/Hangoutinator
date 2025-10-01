//! src/meetup/scrape.rs
#![allow(unused)]

use std::collections::HashMap;

use scraper::{Html, Selector};
use serde_json::{Map, Value, from_str};

use crate::meetup::structure::{Event, FieldType, Member, PhotoInfo, Venue};

// using `consts` instead of `.env` vars.
// NOTE: If watched groups are to be changed from the discord server (by admins),
// a db needs to be used.
const MEETUP_START_URL: &str = "https://meetup.com/";
const MEETUP_END_URL: &str = "/events/?type=upcoming";
const WATCHED_GROUPS: [&str; 2] = ["gwinnett-hangouts", "roswell-and-alpharetta-20s-30s"];

/// used to define a `MeetupManager` as having meetup data
pub struct Populated;
/// used to define a `MeetupManager` as not having any meetup data
pub struct Unpopulated;

/// Manage scraped meetup data
pub struct MeetupManager<State = Unpopulated> {
    // TODO: make separate `Events`, `Members`, etc. section?
    groups_json: HashMap<String, String>,
    watched_groups: Vec<String>,
    state: std::marker::PhantomData<State>,
}

impl MeetupManager {
    fn from() -> Self {
        MeetupManager {
            groups_json: HashMap::default(),
            watched_groups: Vec::from(WATCHED_GROUPS.map(|g| g.to_owned())),
            state: Default::default(),
        }
    }
}

/// methods available to an unpopulated `MeetupManager`
impl MeetupManager<Unpopulated> {
    /// populates `MeetupManager` with meetup data
    pub fn populate(self) -> MeetupManager<Populated> {
        MeetupManager {
            groups_json: self.populate_group_json(),
            watched_groups: self.watched_groups,
            state: std::marker::PhantomData::<Populated>,
        }
    }

    /// populates the json for all meetup groups
    fn populate_group_json(&self) -> HashMap<String, String> {
        let mut groups = HashMap::new();
        for group in self.watched_groups.clone().iter() {
            let json = &self.fetch_json(group).unwrap();
            groups.insert(group.to_owned(), json.to_string());
        }
        groups
    }
}

/// methods available to a populated `MeetupManager`
impl MeetupManager<Populated> {
    /// replaces all json for all meetup groups
    pub fn update_all(&mut self) {
        for group in self.watched_groups.clone().iter() {
            &self.update_one(group);
        }
    }

    /// replaces the json for a specific meetup group
    fn update_one(&mut self, group: &str) {
        let json = &self.fetch_json(group).unwrap();
        &self.groups_json.insert(group.to_string(), json.to_string());
    }

    /// gets the props map containing all events, members, groups, venues, etc.
    /// This function is specific to the meetup JSON data scraped from
    /// their web page. If they change the structure, this will no longer work.
    fn isolate_props(&self, group: &str) -> Map<String, Value> {
        let json = &self.groups_json.get(group).unwrap();
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
    fn extract_fields(&self, props: &Map<String, Value>, partial: &str) -> Vec<Value> {
        props
            .iter()
            .filter_map(|(k, v)| match k.find(partial) {
                Some(_) => Some(v.to_owned()),
                _ => None,
            })
            .collect::<Vec<Value>>()
    }

    fn extract_event() -> Event {
        todo!()
    }
    fn extract_member() -> Member {
        todo!()
    }
    fn extract_venue() -> Venue {
        todo!()
    }
    fn extract_photo() -> PhotoInfo {
        todo!()
    }

    /// get all upcoming events in a given meetup group
    pub fn get_events(&self, group: &str) -> Vec<Event> {
        todo!()
    }
}

/// methods available to any `MeetupManager`
impl<State> MeetupManager<State> {
    /// fetches the JSON data containing meetup events for a particular group
    /// given the URL for that groups upcoming events page
    fn fetch_json(&self, group: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = self.build_url(group);
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
        let json = self.strip_outer_html(script);
        Ok(json.to_string())
    }

    /// builds the url to a groups upcoming events page given the name of the group
    fn build_url(&self, group: &str) -> String {
        format!("{}{}{}", MEETUP_START_URL, group, MEETUP_END_URL)
    }

    /// removes outer html tags (assumes no inner html tags)
    /// NOTE: could be problematic depending on input string
    fn strip_outer_html(&self, html: String) -> String {
        html.split(">").collect::<Vec<&str>>()[1]
            .split("<")
            .collect::<Vec<&str>>()[0]
            .to_string()
    }
}

/// extracts JSON k/v pairs matching the key to a partial string
/// Used to deal with ridiculously named JSON fields
// TODO: Move this method onto the `MeetupManager` struct
fn extract_fields(map: &Value, field_match: &str) -> Vec<(String, Value)> {
    map.as_object()
        .unwrap()
        .iter()
        .filter_map(|(k, v)| match k.find(field_match) {
            Some(_) => Some((k.to_owned(), v.to_owned())),
            _ => None,
        })
        .collect::<Vec<(String, Value)>>()
}

/// extracts an event matching the key to a partial string
/// Used to deal with ridiculously named JSON fields
// TODO: Move this method onto the `MeetupManager` struct
fn extract_event(map: &Value, field_match: &str) -> Event {
    let extracted = &extract_fields(map, field_match)[0].1.to_string();
    from_str(extracted).expect("Could not deserialize json into type `Event`")
}
