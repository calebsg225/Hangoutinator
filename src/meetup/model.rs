//! src/meetup/model.rs
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
/// eg. `Event:123456789` or `Event:xilsndkxcksla`
#[derive(Deserialize)]
pub struct Event {
    __typename: String,
    id: String, // id could be a string of characters instead of a string of digits
    title: String,
    eventUrl: String,
    description: String,
    #[serde(deserialize_with = "string_from_sub_ref")]
    group: String, // points to `Group:` prop
    #[serde(deserialize_with = "string_from_sub_ref")]
    creatorMember: String, // points to `Member:` prop
    #[serde(deserialize_with = "string_vec_from_sub_member_vec")]
    eventHosts: Vec<String>,
    #[serde(deserialize_with = "string_from_sub_ref")]
    venue: String, // points to `Venue:` prop
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    dateTime: DateTime<FixedOffset>,
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    createdTime: DateTime<FixedOffset>,
    #[serde(deserialize_with = "datetime_fixed_offset_from_str")]
    endTime: DateTime<FixedOffset>,
    #[serde(deserialize_with = "usize_from_sub_count")]
    going: usize, // rsvp count
    #[serde(deserialize_with = "string_from_sub_ref")]
    featuredEventPhoto: String, // points to `PhotoInfo:` prop
}

/// data structure matching meetup `Venue:` prop
/// eg. `Venue:123456789`
#[derive(Deserialize)]
pub struct Venue {
    __typename: String,
    id: String, // id could be a string of characters instead of a string of digits
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
    id: String, // id could be a string of characters instead of a string of digits
    name: String,
    #[serde(deserialize_with = "string_from_sub_ref")]
    memberPhoto: String, // ref points to a meetup 'PhotoInfo:' prop
}

/// data structure matching meetup `PhotoInfo:` prop
/// eg. `PhotoInfo:123456789`
#[derive(Deserialize)]
pub struct PhotoInfo {
    __typename: String,
    id: String, // id could be a string of characters instead of a string of digits
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
    memberId: String, // id could be a string of characters instead of a string of digits
}

/// used to comply with meetup json data structure.
/// meetup uses this to count the number of events or members
/// where needed
#[derive(Deserialize)]
pub struct SubCount {
    __typename: String,
    totalCount: usize,
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
    let sub_ref: SubRef = Deserialize::deserialize(deserializer)?;
    Ok(sub_ref.__ref)
}

/// allows serde to deserialize a usize from `SubCount` taken from the
/// JSON data
fn usize_from_sub_count<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let sub_count: SubCount = Deserialize::deserialize(deserializer)?;
    Ok(sub_count.totalCount)
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
        .map(|m| m.memberId.clone())
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
        let de_member =
            from_str::<Member>(sample_member).expect("Could not deserialize string into `Member`.");
        assert_eq!(de_member.id, "123456789");
        assert_eq!(de_member.memberPhoto, "PhotoInfo:123456789");
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
            from_str::<Venue>(sample_venue).expect("Could not deserialize string into `Venue`.");
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
        assert_eq!(de_photo_info.highResUrl, "https://non.ya/business");
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
        let de_event =
            from_str::<Event>(sample_event).expect("Could not deserialize string into `Event`.");
        assert_eq!(de_event.id, "999888777");
        assert_eq!(de_event.title, "IRS Audit");
        assert_eq!(de_event.group, "Group:90909090");
        assert_eq!(de_event.eventHosts.len(), 2);
        assert_eq!(
            de_event.createdTime.to_utc(),
            DateTime::parse_from_rfc3339("2020-08-28T09:17:46-04:00")
                .unwrap()
                .to_utc()
        );
        assert_eq!(de_event.going, 42);
    }
}
