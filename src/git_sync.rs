mod hook;
mod persist;
pub use self::{
    hook::{GitPushPayload, PushKind},
    persist::{PersistMode, Persistable},
};
