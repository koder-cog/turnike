use crate::quota::QuotaInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    Offline,

    CaptiveLocked,

    Authenticated(QuotaInfo),
}
