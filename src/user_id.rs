use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies a user. They can be identified by their Slack id, or by an id that we coined.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UserId {
    /// A user known only by an id we coined for them.
    Uuid(Uuid),
    /// A user known only by their slack id.
    Slack(String),
    /// A user known by both their slack id and their uuid.
    Both { uuid: Uuid, slack: String },
}
impl UserId {
    /// Returns an ID that we coined for a user, if available.
    pub fn uuid(&self) -> Option<Uuid> {
        match self {
            UserId::Uuid(uuid) | UserId::Both { uuid, .. } => Some(*uuid),
            _ => None,
        }
    }
    pub fn uuid_or_else(&self, f: impl FnOnce(&str) -> Uuid) -> Uuid {
        match self {
            UserId::Uuid(uuid) | UserId::Both { uuid, .. } => *uuid,
            UserId::Slack(slack) => f(slack),
        }
    }
    /// Returns a slack id for a user, if available.
    pub fn slack(&self) -> Option<&str> {
        match self {
            UserId::Slack(slack) | UserId::Both { slack, .. } => Some(slack),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const USER_1: &'static str = "U1";
    lazy_static::lazy_static! {
        static ref UUID: Uuid = Uuid::new_v4();
    }

    #[test]
    fn slack_id_fetching() {
        let s = UserId::Slack(USER_1.to_string());
        assert_eq!(s.uuid(), None, "slack only id should not have uuid");
        assert_eq!(
            s.slack(),
            Some(USER_1),
            "slack only id doesn't store user properly"
        );
    }

    #[test]
    fn uuid_id_fetching() {
        let e = UserId::Uuid(*UUID);
        assert_eq!(
            e.uuid(),
            Some(*UUID),
            "uuid only id doesn't store uuid properly"
        );
        assert_eq!(e.slack(), None, "uuid only id shouldn't have slack");
    }

    #[test]
    fn both_id_fetching() {
        let both = UserId::Both {
            slack: USER_1.to_string(),
            uuid: *UUID,
        };
        assert_eq!(
            both.slack(),
            Some(USER_1),
            "both id doesn't store slack properly"
        );

        assert_eq!(
            both.uuid(),
            Some(*UUID),
            "both id doesn't store uuid properly"
        );
    }
}
