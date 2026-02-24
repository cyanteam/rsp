pub mod db;
pub mod request;
pub mod response;

pub use db::Database;
pub use request::{escape_html, Cookies, Headers, Params, Request};
pub use response::ResponseControl;

thread_local! {
    static CURRENT_REQUEST: std::cell::RefCell<Option<Request>> = std::cell::RefCell::new(None);
    static RESPONSE_CONTROL: std::cell::RefCell<ResponseControl> = std::cell::RefCell::new(ResponseControl::new());
}

pub fn set_request(req: Request) {
    CURRENT_REQUEST.with(|r| *r.borrow_mut() = Some(req));
    RESPONSE_CONTROL.with(|r| *r.borrow_mut() = ResponseControl::new());
}

pub fn req() -> Request {
    CURRENT_REQUEST.with(|r| r.borrow().clone().unwrap_or_default())
}

pub fn clear_request() {
    CURRENT_REQUEST.with(|r| *r.borrow_mut() = None);
}

pub fn get_response_control() -> ResponseControl {
    RESPONSE_CONTROL.with(|r| r.borrow().clone())
}

pub fn header(status_code: u16) {
    RESPONSE_CONTROL.with(|r| r.borrow_mut().status_code = status_code);
}

pub fn header_url(url: &str) {
    RESPONSE_CONTROL.with(|r| {
        r.borrow_mut().redirect = Some(url.to_string());
        r.borrow_mut().status_code = 302;
    });
}

pub fn SetCookie(name: &str, value: &str, max_age: i64) {
    RESPONSE_CONTROL.with(|r| {
        r.borrow_mut()
            .cookies
            .push((name.to_string(), value.to_string(), max_age));
    });
}

pub fn CleanCookie(name: &str) {
    RESPONSE_CONTROL.with(|r| {
        r.borrow_mut()
            .cookies
            .push((name.to_string(), "".to_string(), -1));
    });
}
