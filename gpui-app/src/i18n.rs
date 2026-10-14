//! Minimal bilingual dictionary (en + th), ported from `src/lib/i18n/`.
//!
//! Unknown keys fall back to the key itself so missing translations are
//! obvious in the UI rather than silently empty.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Locale {
    En,
    Th,
}

impl Locale {
    /// Detect from the OS locale (`th-*` -> Thai, everything else -> English).
    pub fn detect() -> Self {
        match sys_locale::get_locale() {
            Some(locale) if locale.to_ascii_lowercase().starts_with("th") => Locale::Th,
            _ => Locale::En,
        }
    }
}

#[derive(Clone, Copy)]
pub struct I18n {
    pub locale: Locale,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub fn t(&self, key: &'static str) -> &'static str {
        match self.locale {
            Locale::En => en(key),
            Locale::Th => th(key),
        }
    }
}

fn en(key: &'static str) -> &'static str {
    match key {
        "file" => "File",
        "edit" => "Edit",
        "view" => "View",
        "help" => "Help",
        "new" => "New",
        "open" => "Open...",
        "save" => "Save",
        "save_as" => "Save As...",
        "close_tab" => "Close Tab",
        "quit" => "Quit",
        "cut" => "Cut",
        "copy" => "Copy",
        "paste" => "Paste",
        "select_all" => "Select All",
        "undo" => "Undo",
        "redo" => "Redo",
        "find" => "Find...",
        "find_replace" => "Find & Replace...",
        "toggle_theme" => "Toggle Theme",
        "increase_font" => "Increase Font Size",
        "decrease_font" => "Decrease Font Size",
        "reset_font" => "Reset Font Size",
        "about" => "About DevNote",
        "welcome_title" => "DevNote",
        "welcome_subtitle" => "A fast, distraction-free notepad written in pure Rust.",
        "new_file" => "New File",
        "open_file" => "Open File",
        "recent_files" => "Recent",
        "no_recent" => "No recent files",
        "shortcuts_hint" => {
            "Ctrl+N new · Ctrl+O open · Ctrl+S save · Ctrl+F find · Ctrl+W close tab"
        }
        "untitled" => "untitled",
        "unsaved_title" => "Unsaved changes",
        "unsaved_body" => "You have unsaved files. Save before closing?",
        "save_all" => "Save All",
        "dont_save" => "Don't Save",
        "cancel" => "Cancel",
        "close" => "Close",
        "find_placeholder" => "Find",
        "replace_placeholder" => "Replace",
        "replace" => "Replace",
        "replace_all" => "Replace all",
        "next" => "Next",
        "prev" => "Previous",
        "case_sensitive" => "Match case",
        "use_regex" => "Regex",
        "no_results" => "No results",
        "words" => "words",
        "chars" => "chars",
        "ready" => "Ready",
        "error_open" => "Could not open file",
        "error_save" => "Could not save file",
        "error_too_large" => "File is too large (limit 200 MB)",
        "error_binary" => "Refusing to open: file appears to be binary",
        "error_low_confidence" => "Encoding may be inaccurate - save may change the file",
        "file_read_only" => "Large file - read-only preview",
        "matches" => "matches",
        _ => key,
    }
}

fn th(key: &'static str) -> &'static str {
    match key {
        "file" => "ไฟล์",
        "edit" => "แก้ไข",
        "view" => "มุมมอง",
        "help" => "ช่วยเหลือ",
        "new" => "สร้างใหม่",
        "open" => "เปิด...",
        "save" => "บันทึก",
        "save_as" => "บันทึกเป็น...",
        "close_tab" => "ปิดแท็บ",
        "quit" => "ออก",
        "cut" => "ตัด",
        "copy" => "คัดลอก",
        "paste" => "วาง",
        "select_all" => "เลือกทั้งหมด",
        "undo" => "ย้อนกลับ",
        "redo" => "ทำซ้ำ",
        "find" => "ค้นหา...",
        "find_replace" => "ค้นหาและแทนที่...",
        "toggle_theme" => "สลับธีม",
        "increase_font" => "เพิ่มขนาดตัวอักษร",
        "decrease_font" => "ลดขนาดตัวอักษร",
        "reset_font" => "รีเซ็ตขนาดตัวอักษร",
        "about" => "เกี่ยวกับ DevNote",
        "welcome_title" => "DevNote",
        "welcome_subtitle" => "โปรแกรมจดบันทึกน้ำหนักเบา เขียนด้วย Rust ล้วน",
        "new_file" => "ไฟล์ใหม่",
        "open_file" => "เปิดไฟล์",
        "recent_files" => "เปิดล่าสุด",
        "no_recent" => "ยังไม่มีไฟล์ล่าสุด",
        "shortcuts_hint" => "Ctrl+N ใหม่ · Ctrl+O เปิด · Ctrl+S บันทึก · Ctrl+F ค้นหา · Ctrl+W ปิดแท็บ",
        "untitled" => "ไม่มีชื่อ",
        "unsaved_title" => "มีการเปลี่ยนแปลงที่ยังไม่บันทึก",
        "unsaved_body" => "คุณมีไฟล์ที่ยังไม่ได้บันทึก ต้องการบันทึกก่อนปิดหรือไม่?",
        "save_all" => "บันทึกทั้งหมด",
        "dont_save" => "ไม่บันทึก",
        "cancel" => "ยกเลิก",
        "close" => "ปิด",
        "find_placeholder" => "ค้นหา",
        "replace_placeholder" => "แทนที่",
        "replace" => "แทนที่",
        "replace_all" => "แทนที่ทั้งหมด",
        "next" => "ถัดไป",
        "prev" => "ก่อนหน้า",
        "case_sensitive" => "ตรงตัวพิมพ์",
        "use_regex" => "Regex",
        "no_results" => "ไม่พบผลลัพธ์",
        "words" => "คำ",
        "chars" => "ตัวอักษร",
        "ready" => "พร้อม",
        "error_open" => "ไม่สามารถเปิดไฟล์ได้",
        "error_save" => "ไม่สามารถบันทึกไฟล์ได้",
        "error_too_large" => "ไฟล์ใหญ่เกินไป (จำกัด 200 MB)",
        "error_binary" => "ปฏิเสธการเปิด: ไฟล์ดูเหมือนเป็นไบนารี",
        "error_low_confidence" => "การตรวจจับ encoding อาจไม่แม่นยำ",
        "file_read_only" => "ไฟล์ขนาดใหญ่ - เปิดแบบอ่านอย่างเดียว",
        "matches" => "รายการ",
        _ => key,
    }
}
