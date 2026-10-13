//! Port of eclipse's `Animation`.

use super::button::AnimCallback;

#[derive(Clone, Default)]
pub struct Animation {
    pub buttonname: String,
    pub movetype: i32,
    pub source_x: i32,
    pub source_y: i32,
    pub target_x: i32,
    pub target_y: i32,
    pub d_x: f32,
    pub d_y: f32,
    pub source_w: i32,
    pub source_h: i32,
    pub target_w: i32,
    pub target_h: i32,
    pub d_w: f32,
    pub d_h: f32,
    pub target_c: Option<String>,
    pub d_c: f32,
    pub time: i32,
    pub starttime: i32,
    pub finishtime: i32,
    pub callback: Option<AnimCallback>,
}

impl std::fmt::Debug for Animation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Animation({:?} -> ({},{}) {}x{} caption={:?} {}..{}ms)",
            self.buttonname, self.target_x, self.target_y, self.target_w, self.target_h, self.target_c, self.starttime, self.finishtime
        )
    }
}
