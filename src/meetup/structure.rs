//! src/meetup/structure.rs
#![allow(unused)]
#![allow(non_snake_case)]

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Deserializer, de};
use std::collections::BTreeMap;

pub enum FieldType {
    Event(Event),
    Member(Member),
    PhotoInfo(PhotoInfo),
    Venue(Venue),
}

#[derive(Deserialize)]
pub struct MeetupEvents {
    events: BTreeMap<String, Event>,
}

/// data structure matching meetup `Event:` prop
/// eg. `Event:123456789`
#[derive(Deserialize)]
pub struct Event {
    __typename: String,
    id: String,
    title: String,
    eventUrl: String,
    description: String,
    group: SubRef,
    creatorMember: SubRef,
    eventHosts: Vec<SubMember>,
    venue: SubRef, // points to `Venue:` prop
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    dateTime: DateTime<FixedOffset>,
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    createdTime: DateTime<FixedOffset>,
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    endTime: DateTime<FixedOffset>,
    going: SubCount,            // rsvp count
    featuredEventPhoto: SubRef, // points to `PhotoInfo:` prop
    rsvpState: String,
}

/// data structure matching meetup `Venue:` prop
/// eg. `Venue:123456789`
#[derive(Deserialize)]
pub struct Venue {
    __typename: String,
    id: String,
    name: String,
    address: String,
    city: String,
    state: String,
    country: String,
}

/// data structure matching meetup `Member:` prop
/// eg. `Member:123456789`
#[derive(Deserialize)]
pub struct Member {
    __typename: String,
    id: String,
    name: String,
    memberPhoto: SubRef, // ref points to a meetup 'PhotoInfo:' prop
}

/// data structure matching meetup `PhotoInfo:` prop
/// eg. `PhotoInfo:123456789`
#[derive(Deserialize)]
pub struct PhotoInfo {
    __typename: String,
    id: String,
    highResUrl: String,
}

/// used to comply with meetup json data structure.
/// a ref points to a specific meetup prop,
/// eg. `Member:123456789`, `Event:123456789`, `Venue:123456789`
#[derive(Deserialize)]
pub struct SubRef {
    __ref: String,
}

/// used to comply with meetup json data structure.
/// contains the id of a member, eg. `123456789`
#[derive(Deserialize)]
pub struct SubMember {
    __typename: String,
    memberId: String,
}

/// used to comply with meetup json data structure.
/// meetup uses this to count the number of events or members
/// where needed
#[derive(Deserialize)]
pub struct SubCount {
    __typename: String,
    totalCount: usize,
}

/// allows serde to deserialize a string with assumed datetime format
/// `RFC 3339` directly into `chrono::Datetime<chrono::FixedOffset>>`
/// NOTE: All dates found in the meetup data is in `RFC 3339` format
fn datetime_fixed_offset_from_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s).map_err(de::Error::custom)
}
