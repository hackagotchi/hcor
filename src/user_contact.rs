use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UserContact {
    /// A user known only by their email.
    Email(String),
    /// A user known only by their slack id.
    Slack(String),
    /// A user known by both their slack id and their email.
    Both { email: String, slack: String },
}
impl UserContact {
    /// Returns an email for a user, if available.
    pub fn email(&self) -> Option<&str> {
        Some(match self {
            UserContact::Email(s) => s,
            UserContact::Both { email, .. } => email,
            _ => return None,
        })
    }
    /// Returns a slack id for a user, if available.
    pub fn slack(&self) -> Option<&str> {
        Some(match self {
            UserContact::Slack(s) => s,
            UserContact::Both { slack, .. } => slack,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const USER_1: &'static str = "U1";
    const USER_2: &'static str = "U2";
    const USER_3: &'static str = "U3";

    #[test]
    fn slack_contact_fetching() {
        let s = UserContact::Slack(USER_1.to_string());
        assert_eq!(s.email(), None, "slack only contact should not have email");
        assert_eq!(
            s.slack(),
            Some(USER_1),
            "slack only contact doesn't store user properly"
        );
    }

    #[test]
    fn email_contact_fetching() {
        let e = UserContact::Email(USER_2.to_string());
        assert_eq!(
            e.email(),
            Some(USER_2),
            "email only contact doesn't store email properly"
        );
        assert_eq!(e.slack(), None, "email only contact shouldn't have slack");
    }

    #[test]
    fn both_contact_fetching() {
        let both = UserContact::Both {
            slack: USER_1.to_string(),
            email: USER_3.to_string(),
        };
        assert_eq!(
            both.slack(),
            Some(USER_1),
            "both contact doesn't store slack properly"
        );

        assert_eq!(
            both.email(),
            Some(USER_3),
            "both contact doesn't store email properly"
        );
    }
}
