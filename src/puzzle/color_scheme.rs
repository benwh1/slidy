use palette::rgb::Rgb;

trait ColorScheme {
    fn color(&self, label: usize, num_labels: usize) -> Rgb;
}

struct ColorList {
    colors: Vec<Rgb>,
}

impl ColorScheme for ColorList {
    fn color(&self, label: usize, _num_labels: usize) -> Rgb {
        self.colors[label % self.colors.len()]
    }
}
