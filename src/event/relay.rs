use super::{event::Event, single::single::Single as SingleEvent};

pub struct RelayIterator<'a, T> {
    relay: &'a T,
    index: usize,
}

macro_rules! define_relay {
    ($t:ident) => {
        pub struct $t<'a> {
            start: &'a SingleEvent<'a>,
        }
    };
}

macro_rules! impl_relay {
    ($t:ident) => {
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
}

macro_rules! create_relay {
    ($t:ident) => {
        define_relay!($t);
        impl_relay!($t);
    };
    ($t:ident, $($t2:ident),+ $(,)?) => {
        create_relay!($t);
        create_relay!($($t2),+);
    }
}

create_relay!(
    Single,
    WidthRelay,
    HeightRelay,
    SquareRelay,
    WidthHeightRelay,
);

pub struct Marathon<'a> {
    start: &'a SingleEvent<'a>,
    length: usize,
}

impl_relay!(Marathon);

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

impl<'a> Iterator for RelayIterator<'a, SquareRelay<'a>> {
    type Item = SingleEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let single = if self.index < self.relay.start.width.min(self.relay.start.height) {
            let mut single = self.relay.start.clone();
            single.width -= self.index;
            single.height -= self.index;
            Some(single)
        } else {
            None
        };
        self.index += 1;
        single
    }
}

impl<'a> Iterator for RelayIterator<'a, WidthHeightRelay<'a>> {
    type Item = SingleEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (w, h) = (self.relay.start.width, self.relay.start.height);
        let single = if self.index < (w - 1) * (h - 1) {
            let mut single = self.relay.start.clone();
            single.width -= self.index % (w - 1);
            single.height -= self.index / (w - 1);
            Some(single)
        } else {
            None
        };
        self.index += 1;
        single
    }
}

impl<'a> Iterator for RelayIterator<'a, Marathon<'a>> {
    type Item = SingleEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let single = if self.index < self.relay.length {
            Some(self.relay.start.clone())
        } else {
            None
        };
        self.index += 1;
        single
    }
}
