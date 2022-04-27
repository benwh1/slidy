use palette::rgb::Rgb;
use thiserror::Error;

trait ColorScheme {
    fn color(&self, label: usize, num_labels: usize) -> Rgb;
}

#[derive(Error, Debug)]
enum ColorSchemeError {
    #[error("EmptyColorList: color list must be non-empty")]
    EmptyColorList,
}

struct ColorList {
    colors: Vec<Rgb>,
}

impl ColorList {
    pub fn new(colors: Vec<Rgb>) -> Result<Self, ColorSchemeError> {
        if colors.is_empty() {
            Err(ColorSchemeError::EmptyColorList)
        } else {
            Ok(Self { colors })
        }
    }
}

impl ColorScheme for ColorList {
    fn color(&self, label: usize, _num_labels: usize) -> Rgb {
        self.colors[label % self.colors.len()]
    }
}
