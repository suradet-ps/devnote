use std::sync::Mutex;

pub struct RecentFilesState(pub Mutex<Vec<String>>);
