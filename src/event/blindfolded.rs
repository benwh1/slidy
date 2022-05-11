use super::{event::Event, single::single::Single};

pub struct Blindfolded<'a> {
    pub single: &'a Single<'a>,
}

impl<'a> Event for Blindfolded<'a> {}
