#[derive(Debug, Clone, Default)]
pub struct ResponseControl {
    pub status_code: u16,
    pub redirect: Option<String>,
    pub cookies: Vec<(String, String, i64)>,
    pub headers: Vec<(String, String)>,
}

impl ResponseControl {
    pub fn new() -> Self {
        Self {
            status_code: 200,
            redirect: None,
            cookies: Vec::new(),
            headers: Vec::new(),
        }
    }

    pub fn add_header(&mut self, name: String, value: String) {
        self.headers.push((name, value));
    }
}
