use std::time::Duration;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct SuspendUnawareInstant {}
impl SuspendUnawareInstant {
    pub fn now() -> SuspendUnawareInstant {
        unimplemented!("This platform is not supported by the suspend-time library!");
    }
}
