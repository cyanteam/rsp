use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

static GLOBAL_STATE: Lazy<GlobalState> = Lazy::new(GlobalState::new);

pub type LazyInit = Box<dyn Fn() -> Arc<dyn std::any::Any + Send + Sync> + Send + Sync>;

pub struct GlobalState {
    data: RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>,
    initializers: RwLock<
        HashMap<String, Box<dyn Fn() -> Arc<dyn std::any::Any + Send + Sync> + Send + Sync>>,
    >,
}

impl GlobalState {
    fn new() -> Self {
        GlobalState {
            data: RwLock::new(HashMap::new()),
            initializers: RwLock::new(HashMap::new()),
        }
    }

    pub fn instance() -> &'static Self {
        &GLOBAL_STATE
    }

    pub fn get_or_init<T: 'static + Clone + Send + Sync>(
        &self,
        key: &str,
        init: impl Fn() -> T + Send + Sync + 'static,
    ) -> Arc<T> {
        {
            let data = self.data.read();
            if let Some(value) = data.get(key) {
                if let Ok(typed) = value.clone().downcast::<T>() {
                    return typed;
                }
            }
        }

        let mut data = self.data.write();
        let value = Arc::new(init());
        data.insert(
            key.to_string(),
            value.clone() as Arc<dyn std::any::Any + Send + Sync>,
        );
        value
    }

    pub fn set<T: 'static + Send + Sync>(&self, key: &str, value: Arc<T>) {
        let mut data = self.data.write();
        data.insert(
            key.to_string(),
            value as Arc<dyn std::any::Any + Send + Sync>,
        );
    }

    pub fn get<T: 'static + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        let data = self.data.read();
        data.get(key).and_then(|v| v.clone().downcast::<T>().ok())
    }

    pub fn remove(&self, key: &str) {
        let mut data = self.data.write();
        data.remove(key);
    }

    pub fn clear(&self) {
        let mut data = self.data.write();
        data.clear();
    }

    pub fn contains(&self, key: &str) -> bool {
        let data = self.data.read();
        data.contains_key(key)
    }
}

#[no_mangle]
pub extern "C" fn rsp_state_get_or_init(
    key: *const std::os::raw::c_char,
    init_fn: extern "C" fn() -> *mut std::os::raw::c_void,
) -> *mut std::os::raw::c_void {
    let key_str = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };

    let state = GlobalState::instance();

    if !state.contains(&key_str) {
        let ptr = init_fn();
        if !ptr.is_null() {
            state.set(&key_str, Arc::new(ptr as usize));
        }
    }

    if let Some(arc) = state.get::<usize>(&key_str) {
        Arc::into_raw(arc) as *mut std::os::raw::c_void
    } else {
        std::ptr::null_mut()
    }
}
