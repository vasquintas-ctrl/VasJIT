//! GenericFrame GPU model (Hollywood / Maxwell stubs).
pub mod hollywood;
pub mod maxwell;

pub struct GenericFrame{
    pub width:u32, pub height:u32, pub draws:usize,
}
impl GenericFrame{ pub fn empty()->Self{ Self{width:0,height:0,draws:0} } }
