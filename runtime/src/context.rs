use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl RequestContext {
    pub fn new() -> Self {
        RequestContext {
            method: "GET".to_string(),
            path: "/".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn query(&self, key: &str) -> Option<&String> {
        self.query.get(key)
    }

    pub fn header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

thread_local! {
    static CURRENT_CONTEXT: std::cell::RefCell<Option<RequestContext>> = std::cell::RefCell::new(None);
}

pub fn set_context(ctx: RequestContext) {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(ctx);
    });
}

pub fn get_context() -> Option<RequestContext> {
    CURRENT_CONTEXT.with(|c| c.borrow().clone())
}

pub fn clear_context() {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = None;
    });
}
