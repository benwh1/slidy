use super::single::Single as SingleSolve;

pub struct RelayIterator<'a, T> {
    relay: &'a T,
    index: usize,
}

pub struct Single<'a> {
    start: &'a SingleSolve,
}

impl<'a> Single<'a> {
    pub fn iter(&self) -> RelayIterator<Single> {
        RelayIterator {
            relay: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for RelayIterator<'a, Single<'a>> {
    type Item = SingleSolve;

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
