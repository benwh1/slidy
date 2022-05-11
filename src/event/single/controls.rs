#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Controls {
    Keyboard,
    KeyboardExtended,
    Mouse,
    Tablet,
    Touchscreen,
}
