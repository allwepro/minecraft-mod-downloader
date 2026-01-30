use crate::common::prefabs::notification_window::Notification;

pub struct FailedNotification {
    pub title: String,
    pub reason: String,
}

impl FailedNotification {
    pub fn new(title: &str, reason: &str) -> Self {
        Self {
            title: title.to_string(),
            reason: reason.to_owned(),
        }
    }
}

impl Notification for FailedNotification {
    fn get_title(&self) -> String {
        format!("⚠ {}", self.title)
    }
    fn get_desc(&self) -> String {
        self.reason.clone()
    }
    fn button(&self) -> Option<String> {
        None
    }
}
