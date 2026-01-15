pub(super) struct Base5Table {
    table: Box<[u16]>,
}

impl Base5Table {
    pub(super) fn new() -> Self {
        let mut table = vec![0; 65536];

        for d0 in 0..5 {
            for d1 in 0..5 {
                for d2 in 0..5 {
                    for d3 in 0..5 {
                        let b16 = d0 + 16 * (d1 + 16 * (d2 + 16 * d3));
                        let b5 = d3 + 5 * (d2 + 5 * (d1 + 5 * d0));
                        table[b16] = b5 as u16;
                    }
                }
            }
        }

        Self {
            table: table.into_boxed_slice(),
        }
    }

    pub(super) unsafe fn get_unchecked(&self, index: usize) -> u16 {
        *self.table.get_unchecked(index)
    }
}
