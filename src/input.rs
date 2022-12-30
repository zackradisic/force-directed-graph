pub struct InputState {
    pub dragging: Option<u32>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            dragging: Default::default(),
        }
    }
}
