use super::{event::Event, single::single::Single};
use std::time::Duration;

pub struct TimeAttack<'a> {
    pub single: &'a Single<'a>,
    pub time_limit: Duration,
}

impl<'a> Event for TimeAttack<'a> {}
