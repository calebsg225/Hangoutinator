//! src/meetup/structure.rs
#![allow(unused)]
#![allow(non_snake_case)]

use core::error;
use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct MeetupEvents {
    events: BTreeMap<String, Event>,
}

/// used to comply with meetup json data structure.
/// a ref points to a specific meetup field,
/// eg. `Member:123456789`, `Event:123456789`, `Venue:123456789`
#[derive(Deserialize)]
struct SubRef {
    __ref: String,
}

/// used to comply with meetup json data structure.
/// contains the id of a member, eg. `123456789`
#[derive(Deserialize)]
struct SubMember {
    __typename: String,
    memberId: String,
}

/// used to comply with meetup json data structure.
/// meetup uses this to count the number of events or members
/// where needed
#[derive(Deserialize)]
struct SubCount {
    __typename: String,
    totalCount: usize,
}

/// data structure matching meetup `Event:` field
/// eg. `Event:123456789`
#[derive(Deserialize)]
struct Event {
    __typename: String,
    id: String,
    title: String,
    eventUrl: String,
    description: String,
    group: SubRef,
    creatorMember: SubRef,
    eventHosts: Vec<SubMember>,
    venue: SubRef,       // ref points to a meetup `Venue:` field
    dateTime: String,    // convert to date
    createdTime: String, // convert to date
    endTime: String,     // convert to date
    going: SubCount,     // rsvp count
    featuredEventPhoto: SubRef,
    rsvpState: String,
}

/// data structure matching meetup `Venue:` field
/// eg. `Venue:123456789`
#[derive(Deserialize)]
struct Venue {
    __typename: String,
    id: String,
    name: String,
    address: String,
    city: String,
    state: String,
    country: String,
}

/// data structure matching meetup `Member:` field
/// eg. `Member:123456789`
#[derive(Deserialize)]
struct Member {
    __typename: String,
    id: String,
    name: String,
    memberPhoto: SubRef, // ref points to a meetup 'PhotoInfo:' field
}

/// data structure matching meetup `PhotoInfo:` field
/// eg. `PhotoInfo:123456789`
#[derive(Deserialize)]
struct PhotoInfo {
    __typename: String,
    id: String,
    highResUrl: String,
}
