use palette::rgb::Rgb;

trait ColorScheme {
    fn color(&self, label: usize, num_labels: usize) -> Rgb;
}

struct Monochrome {
    color: Rgb,
}

impl ColorScheme for Monochrome {
    fn color(&self, _label: usize, _num_labels: usize) -> Rgb {
        self.color
    }
}
