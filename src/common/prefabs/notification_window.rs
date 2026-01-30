pub trait Notification: 'static + Send + Sync {
    fn get_title(&self) -> String;
    fn get_desc(&self) -> String;
    fn button(&self) -> Option<String>;
    fn on_click(&mut self) {}
    fn on_close(&mut self) {}
}
