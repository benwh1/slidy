use palette::rgb::Rgb;

trait ColorScheme {
    fn color(&self, label: usize, num_labels: usize) -> Rgb;
}
