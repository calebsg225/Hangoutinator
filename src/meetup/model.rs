//! src/meetup/model.rs
//#![allow(unused)]
//#![allow(non_snake_case)]

use chrono::{DateTime, Datelike, FixedOffset};
use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeOwned},
};

use serde_json::{Map, Value, from_value};
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

/// contains all data deserialized from meetup.com JSON, then uses
/// that data to build `refined` event data in the form of a single
/// `MeetupEvent` type
pub struct MeetupEventBuilder {
    group: RawGroup,
    events: Vec<RawMeetupEvent>,
    members: BTreeMap<String, RawMember>,
    photos: BTreeMap<String, PhotoInfo>,
    venues: BTreeMap<String, RawVenue>,
}

impl MeetupEventBuilder {
    pub fn from(json: Map<String, Value>) -> Self {
        let meetup_data = Self {
            group: extract_fields(&json, "Group:").pop_first().unwrap().1,
            events: extract_sorted_events(&json, "Event:"),
            members: extract_fields(&json, "Member:"),
            photos: extract_fields(&json, "PhotoInfo:"),
            venues: extract_fields(&json, "Venue:"),
        };
        meetup_data
    }
    /// collects 'raw' meetup event data into 'refined' meetup events
    pub fn get_events(&self) -> Vec<MeetupEvent> {
        self.events
            .iter()
            .map(|e| self.refine_event(e.to_owned()))
            .collect()
    }
    /// get a 'raw' meetup member reference
    fn get_member(&self, id: &str) -> Option<&RawMember> {
        self.members.get(id)
    }
    /// get a 'raw' meetup photoinfo reference
    fn get_photoinfo(&self, id: &str) -> Option<&PhotoInfo> {
        self.photos.get(id)
    }
    /// get a 'raw' meetup venue reference
    fn get_venue(&self, id: &str) -> Option<&RawVenue> {
        self.venues.get(id)
    }
    /// turns a 'raw' venue into a 'refined' venue
    fn refine_venue(&self, venue_id: &str) -> Venue {
        let raw_venue = self.get_venue(venue_id).unwrap();
        Venue::from(raw_venue.clone())
    }
    /// turns a 'raw' member into a 'refined' member
    fn refine_member(&self, member_id: &str) -> Member {
        let raw_member = self.get_member(member_id).unwrap();
        let photo = self.get_photoinfo(&raw_member.photo);
        Member::from(raw_member.clone(), photo.cloned())
    }
    /// turns a 'raw' group into a 'refined' group
    fn refine_group(&self, organizer: Member) -> Group {
        Group::from(self.group.clone(), organizer)
    }
    /// turns a 'raw' event into a 'refined' event
    fn refine_event(&self, raw_event: RawMeetupEvent) -> MeetupEvent {
        let organizer = self.refine_member(&self.group.organizer);
        let group = self.refine_group(organizer);
        let creator_member = self.refine_member(&raw_event.creator_member);
        let venue = self.refine_venue(&raw_event.venue);
        let photo = self.get_photoinfo(&raw_event.photo);
        let hosts = raw_event
            .hosts
            .iter()
            .map(|m| self.refine_member(m))
            .collect();
        MeetupEvent::from(
            raw_event,
            group,
            creator_member,
            venue,
            hosts,
            photo.cloned(),
        )
    }
}

/// 'raw' meetup event data, newly converted from the JSON
/// data structure matching meetup `Event:` prop
/// eg. `Event:123456789` or `Event:xilsndkxcksla`
#[derive(Deserialize, Clone)]
struct RawMeetupEvent {
    pub id: String, // id could be a string of characters instead of a string of digits
    pub title: String,
    #[serde(rename = "eventUrl")]
    pub event_url: String,
    pub description: String,
    #[serde(rename = "creatorMember")]
    #[serde(deserialize_with = "string_from_sub_ref")]
    pub creator_member: String, // points to `Member:` prop
    #[serde(rename = "eventHosts")]
    #[serde(deserialize_with = "string_vec_from_sub_member_vec")]
    pub hosts: Vec<String>,
    #[serde(deserialize_with = "string_from_sub_ref")]
    pub venue: String, // points to `Venue:` prop
    #[serde(rename = "dateTime")]
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    pub start_time: DateTime<FixedOffset>,
    #[serde(rename = "createdTime")]
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    pub created_time: DateTime<FixedOffset>,
    #[serde(rename = "endTime")]
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    pub end_time: DateTime<FixedOffset>,
    #[serde(deserialize_with = "usize_from_sub_count")]
    pub going: usize, // rsvp count
    #[serde(rename = "featuredEventPhoto")]
    #[serde(deserialize_with = "string_from_sub_ref")]
    pub photo: String, // points to `PhotoInfo:` prop
}

/// 'refined' meetup event data built from a 'RawMeetupEvent' stuct
#[allow(unused)]
pub struct MeetupEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub event_url: String,
    pub group: Group,
    pub creator_member: Member,
    pub hosts: Vec<Member>,
    pub venue: Venue,
    pub start_time: DateTime<FixedOffset>,
    pub created_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub going: usize,
    pub photo: Option<PhotoInfo>,
}

impl MeetupEvent {
    fn from(
        raw_event: RawMeetupEvent,
        group: Group,
        creator_member: Member,
        venue: Venue,
        hosts: Vec<Member>,
        photo: Option<PhotoInfo>,
    ) -> Self {
        Self {
            id: raw_event.id,
            title: raw_event.title,
            description: raw_event.description,
            event_url: raw_event.event_url,
            group,
            creator_member: creator_member,
            hosts,
            venue,
            start_time: raw_event.start_time,
            created_time: raw_event.created_time,
            end_time: raw_event.end_time,
            going: raw_event.going,
            photo,
        }
    }
    /// generates a unique event hash
    pub fn get_hash(&self) -> u64 {
        let mut state = DefaultHasher::new();
        self.creator_member.id.hash(&mut state);
        self.venue.address.hash(&mut state);
        self.venue.state.hash(&mut state);
        self.title.hash(&mut state);
        self.description.hash(&mut state);
        self.created_time.hash(&mut state);
        state.finish()
    }
    /// generates an event hash to identify duplicate events.
    ///
    /// A duplicate event in this context is an event identical in nature to another
    /// event in a different meetup group, Ex. the same board game event posted in two
    /// or more meetup groups.
    pub fn get_dup_hash(&self) -> u64 {
        let mut state = DefaultHasher::new();
        self.creator_member.id.hash(&mut state);
        self.venue.address.hash(&mut state);
        self.venue.state.hash(&mut state);
        self.start_time.hash(&mut state);
        state.finish()
    }
    /// generate an event hash to identify meetup events that repeat on a weekly basis.
    ///
    /// All meetup events with this hash are considered to be part of the same weekly collection.
    pub fn get_weekly_collection_hash(&self) -> u64 {
        let mut state = DefaultHasher::new();
        self.creator_member.id.hash(&mut state);
        self.venue.address.hash(&mut state);
        self.venue.state.hash(&mut state);
        // make the time and day of the event part of the hash, removing the date
        self.start_time.time().hash(&mut state);
        self.start_time.weekday().hash(&mut state);
        state.finish()
    }
}

/// 'raw' meetup venue data, newly converted from the JSON
/// data structure matching meetup `Venue:` prop
/// eg. `Venue:123456789`
#[derive(Deserialize, Clone)]
struct RawVenue {
    pub id: String, // id could be a string of characters instead of a string of digits
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub country: String,
}

/// 'refined' venue data built from a 'RawVenue' stuct
#[derive(Hash)]
pub struct Venue {
    pub id: String,
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub location: String,
}

impl Venue {
    fn from(rv: RawVenue) -> Self {
        let location = format!(
            "{} {} {} {} {}",
            &rv.name, &rv.address, &rv.city, &rv.state, &rv.country
        );
        Self {
            id: rv.id,
            name: rv.name,
            address: rv.address,
            city: rv.city,
            state: rv.state,
            country: rv.country,
            location,
        }
    }
}

/// 'raw' meetup member data, newly converted from the JSON
/// data structure matching meetup `Member:` prop
/// eg. `Member:123456789`
#[derive(Deserialize, Clone, Debug)]
struct RawMember {
    pub id: String, // id could be a string of characters instead of a string of digits
    pub name: String,
    #[serde(default)]
    #[serde(deserialize_with = "string_from_sub_ref")]
    #[serde(rename = "memberPhoto")]
    pub photo: String, // ref points to a meetup 'PhotoInfo:' prop
}

/// 'refined' member data built from a 'RawMember' stuct
#[derive(Hash)]
pub struct Member {
    pub id: String,
    pub name: String,
    pub photo: Option<PhotoInfo>,
}

impl Member {
    fn from(rm: RawMember, photo: Option<PhotoInfo>) -> Self {
        Self {
            id: rm.id,
            name: rm.name,
            photo,
        }
    }
}

/// 'raw' meetup photo data, newly converted from the JSON
/// no refining is required, therefore no 'raw' identifier
/// data structure matching meetup `PhotoInfo:` prop
/// eg. `PhotoInfo:123456789`
#[derive(Deserialize, Clone, Hash)]
pub struct PhotoInfo {
    pub id: String, // id could be a string of characters instead of a string of digits
    #[serde(rename = "highResUrl")]
    pub high_res_url: String,
}

/// 'raw' meetup group data, newly converted from the JSON
/// data structure matching meetup `Group:` prop
/// eg. `Group:123456789`
#[derive(Deserialize, Clone)]
struct RawGroup {
    pub id: String,
    pub name: String,
    #[serde(rename = "urlname")]
    pub url_name: String,
    #[serde(deserialize_with = "string_from_sub_ref")]
    pub organizer: String,
}

/// 'refined' group data built from a 'RawGroup' stuct
#[derive(Hash)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub url_name: String,
    pub organizer: Member,
}

impl Group {
    fn from(raw_group: RawGroup, organizer: Member) -> Self {
        Group {
            id: raw_group.id,
            name: raw_group.name,
            url_name: raw_group.url_name,
            organizer: organizer,
        }
    }
}

/// used to comply with meetup json data structure.
/// a ref points to a specific meetup prop,
/// eg. `Member:123456789`, `Event:123456789`, `Venue:123456789`
#[derive(Deserialize)]
struct SubRef {
    __ref: String,
}

/// used to comply with meetup json data structure.
/// contains the id of a member, eg. `123456789`
#[derive(Deserialize)]
struct SubMember {
    #[serde(rename = "memberId")]
    pub member_id: String, // id could be a string of characters instead of a string of digits
}

/// used to comply with meetup json data structure.
/// meetup uses this to count the number of events or members
/// where needed
#[derive(Deserialize)]
struct SubCount {
    #[serde(rename = "totalCount")]
    pub total_count: usize,
}

/// Extracts JSON `Value`s whos keys match a partial string.
/// Used for dealing with ridiculously named JSON fields.
///
/// Extracts to rust struct types:
/// - `PhotoInfo`
/// - `RawVenue`
/// - `RawMember`
/// - `RawGroup`
/// - `RawMeetupEvent`
fn extract_fields<T: DeserializeOwned>(
    props: &Map<String, Value>,
    partial: &str,
) -> BTreeMap<String, T> {
    props
        .iter()
        .filter_map(|(k, v)| match k.find(partial) {
            Some(_) => Some((
                //k.strip_prefix(partial).unwrap().to_owned(),
                k.to_owned(),
                from_value::<T>(v.to_owned())
                    .expect(&format!("Could not convert [{k},{v}] `Value` to type `T`.")),
            )),
            _ => None,
        })
        .collect::<BTreeMap<String, T>>()
}

/// Extracts JSON `Event`s and sorts them by earliest date
fn extract_sorted_events(props: &Map<String, Value>, partial: &str) -> Vec<RawMeetupEvent> {
    let mut events = extract_fields::<RawMeetupEvent>(props, partial)
        .iter()
        .map(|(_, v)| v.clone())
        .collect::<Vec<RawMeetupEvent>>();
    // sort meetup events by date (earliest first)
    events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    events
}

/// NOTE: The following functions remove excessive nesting in the meetup JSON
/// data when converting into rust structs

/// allows serde to deserialize a string with assumed datetime format
/// `RFC 3339` directly into `chrono::Datetime<chrono::FixedOffset>>`
/// NOTE: All dates (currently) found in the meetup data are in `RFC 3339` format
fn datetime_fixed_offset_from_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s).map_err(de::Error::custom)
}

/// allows serde to deserialize a string from `SubRef` taken from the
/// JSON data
fn string_from_sub_ref<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let sub_ref: Option<SubRef> = Deserialize::deserialize(deserializer)?;
    Ok(match sub_ref {
        Some(sub) => sub.__ref,
        None => String::new(),
    })
}

/// allows serde to deserialize a usize from `SubCount` taken from the
/// JSON data
fn usize_from_sub_count<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let sub_count: SubCount = Deserialize::deserialize(deserializer)?;
    Ok(sub_count.total_count)
}

/// allows serde to deserialize a `String` vec from `Vec<SubMember>` taken from the
/// JSON data
fn string_vec_from_sub_member_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let members: Vec<SubMember> = Deserialize::deserialize(deserializer)?;
    let members = members
        .iter()
        .map(|m| format!("Member:{}", m.member_id.clone()))
        .collect::<Vec<String>>();
    Ok(members)
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;

    use super::*;

    /// meetup `Member` prop can be converted into `Member` struct
    #[test]
    fn can_deserialize_member() {
        let sample_member = r#"{
            "__typename": "Member",
            "id": "123456789",
            "name": "John Doe",
            "memberPhoto": {
                "__ref": "PhotoInfo:123456789"
            }
        }"#;
        let de_member = from_str::<RawMember>(sample_member)
            .expect("Could not deserialize string into `Member`.");
        assert_eq!(de_member.id, "123456789");
        assert_eq!(de_member.photo, "PhotoInfo:123456789");
        assert_eq!(de_member.name, "John Doe");
    }

    /// meetup `Venue` prop can be converted into `Venue` struct
    #[test]
    fn can_deserialize_venue() {
        let sample_venue = r#"{
            "__typename": "Venue",
            "id": "987654321",
            "name": "Micky D's",
            "address": "420 blvd",
            "city": "Bill",
            "state": "Cosby",
            "country": "Mars"
        }"#;
        let de_venue =
            from_str::<RawVenue>(sample_venue).expect("Could not deserialize string into `Venue`.");
        assert_eq!(de_venue.id, "987654321");
        assert_eq!(de_venue.name, "Micky D's");
        assert_eq!(de_venue.address, "420 blvd");
        assert_eq!(de_venue.city, "Bill");
        assert_eq!(de_venue.state, "Cosby");
        assert_eq!(de_venue.country, "Mars");
    }

    /// meetup `PhotoInfo` prop can be converted into `PhotoInfo` struct
    #[test]
    fn can_deserialize_photo_info() {
        let sample_photo_info = r#"{
            "__typename": "PhotoInfo",
            "id": "000111222",
            "highResUrl": "https://non.ya/business"
        }"#;
        let de_photo_info = from_str::<PhotoInfo>(sample_photo_info)
            .expect("Could not deserialize string into `PhotoInfo`.");
        assert_eq!(de_photo_info.id, "000111222");
        assert_eq!(de_photo_info.high_res_url, "https://non.ya/business");
    }

    /// meetup `Event` prop can be converted into `Event` struct
    #[test]
    fn can_deserialize_event() {
        let sample_event = r#"{
            "__typename": "Event",
            "id": "999888777",
            "title": "IRS Audit",
            "eventUrl": "https://www.irs.com/audit",
            "description": "no money for you",
            "group": {
                "__ref": "Group:90909090"
            },
            "creatorMember": {
                "__ref": "Member:707070707"
            },
            "eventHosts": [
                {
                    "__typename": "EventHost",
                    "memberId": "505050505"
                },
                {
                    "__typename": "EventHost",
                    "memberId": "303030303"
                }
            ],
            "venue": {
                "__ref": "Venue:22222222"
            },
            "dateTime": "2020-01-01T08:15:00-04:00",
            "createdTime": "2020-08-28T09:17:46-04:00",
            "endTime": "2020-01-01T01:00:00-04:00",
            "going": {
                "__typename": "GoingRsvpConnection",
                "totalCount": 42
            },
            "featuredEventPhoto": {
                "__ref": "PhotoInfo:141414141"
            }
        }"#;
        let de_event = from_str::<RawMeetupEvent>(sample_event)
            .expect("Could not deserialize string into `Event`.");
        assert_eq!(de_event.id, "999888777");
        assert_eq!(de_event.title, "IRS Audit");
        assert_eq!(de_event.hosts.len(), 2);
        assert_eq!(
            de_event.created_time.to_utc(),
            DateTime::parse_from_rfc3339("2020-08-28T09:17:46-04:00")
                .unwrap()
                .to_utc()
        );
        assert_eq!(de_event.going, 42);
    }
}
