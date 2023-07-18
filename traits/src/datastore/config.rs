pub struct Forcable<T: Sized> {
    forced: bool,
    value: T,
}

impl<T: Sized> Forcable<T> {
    pub fn new(value: T) -> Self {
        Self {
            forced: false,
            value,
        }
    }
    pub fn new_forced(value: T) -> Self {
        Self {
            forced: true,
            value,
        }
    }
    pub fn get_value(self) -> T {
        self.value
    }
    pub fn is_forced(&self) -> bool {
        self.forced
    }
}