#[derive(Clone)]
pub struct Mouse {
    /// Mouse position in screen coordinates
    pub pos: Option<cgmath::Vector2<f32>>,
    pub last_pos: cgmath::Vector2<f32>,
    pub clicked: bool,
}

impl Default for Mouse {
    fn default() -> Self {
        Self {
            pos: Default::default(),
            last_pos: (0.0, 0.0).into(),
            clicked: Default::default(),
        }
    }
}
