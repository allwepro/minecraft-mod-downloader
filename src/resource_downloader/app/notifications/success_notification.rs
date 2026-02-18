use crate::common::ui::structs::notification_window::Notification;

pub struct SuccessNotification {
    pub title: String,
    pub message: String,
}

impl SuccessNotification {
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_owned(),
        }
    }
}

impl Notification for SuccessNotification {
    fn get_title(&self) -> String {
        format!("✔ {}", self.title)
    }
    fn get_desc(&self) -> String {
        self.message.clone()
    }
    fn button(&self) -> Option<String> {
        None
    }
}
