use super::single::{Event, Single as SingleEvent};

pub struct RelayIterator<'a, T> {
    relay: &'a T,
    index: usize,
}

macro_rules! create_relay {
    ($t:ident) => {
        pub struct $t<'a> {
            start: &'a SingleEvent<'a>,
        }

        impl<'a> $t<'a> {
            pub fn iter(&self) -> RelayIterator<$t> {
                RelayIterator {
                    relay: self,
                    index: 0,
                }
            }
        }

        impl<'a> Event for $t<'a> {}
    };
    ($t:ident, $($t2:ident),+ $(,)?) => {
        create_relay!($t);
        create_relay!($($t2),+);
    }
}

create_relay!(Single, WidthRelay, HeightRelay);

impl<'a> Iterator for RelayIterator<'a, Single<'a>> {
    type Item = SingleEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let single = if self.index == 0 {
            Some(self.relay.start.clone())
        } else {
            None
        };
        self.index += 1;
        single
    }
}

impl<'a> Iterator for RelayIterator<'a, WidthRelay<'a>> {
    type Item = SingleEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let single = if self.index < self.relay.start.width {
            let mut single = self.relay.start.clone();
            single.width -= self.index;
            Some(single)
        } else {
            None
        };
        self.index += 1;
        single
    }
}

impl<'a> Iterator for RelayIterator<'a, HeightRelay<'a>> {
    type Item = SingleEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let single = if self.index < self.relay.start.height {
            let mut single = self.relay.start.clone();
            single.height -= self.index;
            Some(single)
        } else {
            None
        };
        self.index += 1;
        single
    }
}
