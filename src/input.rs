#[derive(Copy, Clone, Debug)]
pub enum DragKind {
    Node(u32),
    EdgeCreation(u32),
}

pub struct InputState {
    pub dragging: Option<DragKind>,
    pub is_ctrl_pressed: bool,
    pub is_lalt_pressed: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            dragging: Default::default(),
            is_ctrl_pressed: false,
            is_lalt_pressed: false,
        }
    }
}
