use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::ptr;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, BadgerError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Status {
    Ok = 0,
    InvalidArgument = 1,
    InvalidHandle = 2,
    NotFound = 3,
    EmptyKey = 4,
    DiscardedTxn = 5,
    DbClosed = 6,
    TxnConflict = 7,
    ReadOnlyTxn = 8,
    InvalidEncryptionKey = 9,
    InvalidValueLogSize = 10,
    Unknown = 11,
}

impl From<Status> for BadgerError {
    fn from(value: Status) -> Self {
        match value {
            Status::InvalidArgument => BadgerError::InvalidArgument("unknown".to_string()),
            Status::InvalidHandle => BadgerError::InvalidHandle,
            Status::NotFound => BadgerError::NotFound,
            Status::EmptyKey => BadgerError::EmptyKey,
            Status::DiscardedTxn => BadgerError::DiscardedTxn,
            Status::DbClosed => BadgerError::DbClosed,
            Status::TxnConflict => BadgerError::TxnConflict,
            Status::ReadOnlyTxn => BadgerError::ReadOnlyTxn,
            Status::InvalidEncryptionKey => BadgerError::InvalidEncryptionKey,
            Status::InvalidValueLogSize => BadgerError::InvalidValueLogSize,
            Status::Unknown => BadgerError::Unknown,
            _ => BadgerError::Unknown,
        }
    }
}

impl From<BadgerError> for Status {
    fn from(value: BadgerError) -> Self {
        match value {
            BadgerError::InvalidArgument(_) => Self::InvalidArgument,
            BadgerError::InvalidHandle => Self::InvalidHandle,
            BadgerError::NotFound => Self::NotFound,
            BadgerError::EmptyKey => Self::EmptyKey,
            BadgerError::DiscardedTxn => Self::DiscardedTxn,
            BadgerError::DbClosed => Self::DbClosed,
            BadgerError::TxnConflict => Self::TxnConflict,
            BadgerError::ReadOnlyTxn => Self::ReadOnlyTxn,
            BadgerError::InvalidEncryptionKey => Self::InvalidEncryptionKey,
            BadgerError::InvalidValueLogSize => Self::InvalidValueLogSize,
            BadgerError::Unknown => Self::Unknown,
        }
    }
}

impl From<i32> for Status {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::InvalidArgument,
            2 => Self::InvalidHandle,
            3 => Self::NotFound,
            4 => Self::EmptyKey,
            5 => Self::DiscardedTxn,
            6 => Self::DbClosed,
            7 => Self::TxnConflict,
            8 => Self::ReadOnlyTxn,
            9 => Self::InvalidEncryptionKey,
            10 => Self::InvalidValueLogSize,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Error, Clone)]
pub enum BadgerError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Not found")]
    NotFound,
    #[error("Empty key")]
    EmptyKey,
    #[error("Discarded transaction")]
    DiscardedTxn,
    #[error("Database closed")]
    DbClosed,
    #[error("Transaction conflict")]
    TxnConflict,
    #[error("Read-only transaction")]
    ReadOnlyTxn,
    #[error("Invalid encryption key")]
    InvalidEncryptionKey,
    #[error("Invalid value log size")]
    InvalidValueLogSize,
    #[error("Unknown error")]
    Unknown,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OpenOptions {
    pub value_dir: String,
    pub in_memory: bool,
    pub encryption_key: Vec<u8>,
    pub index_cache_size: i64,
    pub value_log_file_size: i64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IteratorOptions {
    pub prefix: Option<Vec<u8>>,
    pub start: Option<Vec<u8>>,
    pub end: Option<Vec<u8>>,
    pub reverse: bool,
    pub keys_only: bool,
}

#[derive(Debug)]
pub struct Database {
    handle: u64,
}

impl Database {
    pub fn open(path: impl AsRef<Path>, options: &OpenOptions) -> Result<Self> {
        let dir = to_c_string(
            path.as_ref()
                .to_str()
                .ok_or(BadgerError::InvalidArgument("invalid path".to_string()))?,
            "dir",
        )?;
        let value_dir = to_c_string(&options.value_dir, "value_dir")?;
        let (key_ptr, key_len) = bytes_arg(&options.encryption_key);
        let mut handle = 0_u64;
        let mut err = ptr::null_mut();

        let status = unsafe {
            raw::badger_ffi_db_open(
                dir.as_ptr(),
                value_dir.as_ptr(),
                bool_to_u8(options.in_memory),
                key_ptr,
                key_len,
                options.index_cache_size,
                options.value_log_file_size,
                &mut handle,
                &mut err,
            )
        };
        status_result(status, err)?;

        Ok(Self { handle })
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn is_closed(&self) -> bool {
        self.handle == 0
    }

    pub fn close(&mut self) -> Result<()> {
        if self.handle == 0 {
            return Ok(());
        }

        let mut err = ptr::null_mut();
        let status = unsafe { raw::badger_ffi_db_close(self.handle, &mut err) };
        status_result(status, err)?;
        self.handle = 0;
        Ok(())
    }

    pub fn drop_all(&self) -> Result<()> {
        let mut err = ptr::null_mut();
        let status = unsafe { raw::badger_ffi_db_drop_all(self.handle, &mut err) };
        status_result(status, err)
    }

    pub fn new_txn(&self, read_only: bool) -> Result<Txn> {
        let mut handle = 0_u64;
        let mut err = ptr::null_mut();
        let status = unsafe {
            raw::badger_ffi_db_new_txn(self.handle, bool_to_u8(read_only), &mut handle, &mut err)
        };
        status_result(status, err)?;

        Ok(Txn { handle })
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[derive(Debug)]
pub struct Txn {
    handle: u64,
}

impl Txn {
    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn is_discarded(&self) -> bool {
        self.handle == 0
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let (key_ptr, key_len) = bytes_arg(key);
        let mut value_ptr = ptr::null_mut();
        let mut value_len = 0_usize;
        let mut err = ptr::null_mut();
        let status = unsafe {
            raw::badger_ffi_txn_get(
                self.handle,
                key_ptr,
                key_len,
                &mut value_ptr,
                &mut value_len,
                &mut err,
            )
        };
        status_result(status, err)?;

        Ok(take_bytes(value_ptr, value_len))
    }

    pub fn has(&self, key: &[u8]) -> Result<bool> {
        let (key_ptr, key_len) = bytes_arg(key);
        let mut has = 0_u8;
        let mut err = ptr::null_mut();
        let status =
            unsafe { raw::badger_ffi_txn_has(self.handle, key_ptr, key_len, &mut has, &mut err) };
        status_result(status, err)?;

        Ok(has != 0)
    }

    pub fn set(&self, key: &[u8], value: Option<&[u8]>) -> Result<()> {
        let (key_ptr, key_len) = bytes_arg(key);
        let (value_ptr, value_len) = bytes_arg_opt(value);
        let mut err = ptr::null_mut();
        let status = unsafe {
            raw::badger_ffi_txn_set(
                self.handle,
                key_ptr,
                key_len,
                value_ptr,
                value_len,
                &mut err,
            )
        };
        status_result(status, err)
    }

    pub fn delete(&self, key: &[u8]) -> Result<()> {
        let (key_ptr, key_len) = bytes_arg(key);
        let mut err = ptr::null_mut();
        let status = unsafe { raw::badger_ffi_txn_delete(self.handle, key_ptr, key_len, &mut err) };
        status_result(status, err)
    }

    pub fn commit(&self) -> Result<()> {
        let mut err = ptr::null_mut();
        let status = unsafe { raw::badger_ffi_txn_commit(self.handle, &mut err) };
        status_result(status, err)
    }

    pub fn discard(&mut self) {
        if self.handle == 0 {
            return;
        }

        unsafe { raw::badger_ffi_txn_discard(self.handle) };
        self.handle = 0;
    }

    pub fn iterator(&self, options: &IteratorOptions) -> Result<BadgerIterator> {
        let (prefix_ptr, prefix_len) = bytes_arg_opt(options.prefix.as_deref());
        let (start_ptr, start_len) = bytes_arg_opt(options.start.as_deref());
        let (end_ptr, end_len) = bytes_arg_opt(options.end.as_deref());
        let mut handle = 0_u64;
        let mut err = ptr::null_mut();
        let status = unsafe {
            raw::badger_ffi_txn_iterator(
                self.handle,
                prefix_ptr,
                prefix_len,
                start_ptr,
                start_len,
                end_ptr,
                end_len,
                bool_to_u8(options.reverse),
                bool_to_u8(options.keys_only),
                &mut handle,
                &mut err,
            )
        };
        status_result(status, err)?;

        Ok(BadgerIterator { handle })
    }
}

impl Drop for Txn {
    fn drop(&mut self) {
        self.discard();
    }
}

#[derive(Debug)]
pub struct BadgerIterator {
    handle: u64,
}

impl BadgerIterator {
    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn is_closed(&self) -> bool {
        self.handle == 0
    }

    /// equivalent to go iter.next()
    pub fn has_next(&mut self) -> Result<bool> {
        let mut has = 0_u8;
        let mut err = ptr::null_mut();
        let status = unsafe { raw::badger_ffi_iter_next(self.handle, &mut has, &mut err) };
        status_result(status, err)?;

        Ok(has != 0)
    }

    pub fn key(&self) -> Result<Vec<u8>> {
        let mut key_ptr = ptr::null_mut();
        let mut key_len = 0_usize;
        let mut err = ptr::null_mut();
        let status =
            unsafe { raw::badger_ffi_iter_key(self.handle, &mut key_ptr, &mut key_len, &mut err) };
        status_result(status, err)?;

        Ok(take_bytes(key_ptr, key_len).unwrap_or_default())
    }

    pub fn value(&self) -> Result<Option<Vec<u8>>> {
        let mut value_ptr = ptr::null_mut();
        let mut value_len = 0_usize;
        let mut err = ptr::null_mut();
        let status = unsafe {
            raw::badger_ffi_iter_value(self.handle, &mut value_ptr, &mut value_len, &mut err)
        };
        status_result(status, err)?;

        Ok(take_bytes(value_ptr, value_len))
    }

    pub fn seek(&mut self, key: &[u8]) -> Result<bool> {
        let (key_ptr, key_len) = bytes_arg(key);
        let mut has = 0_u8;
        let mut err = ptr::null_mut();
        let status =
            unsafe { raw::badger_ffi_iter_seek(self.handle, key_ptr, key_len, &mut has, &mut err) };
        status_result(status, err)?;

        Ok(has != 0)
    }

    pub fn reset(&mut self) -> Result<()> {
        let mut err = ptr::null_mut();
        let status = unsafe { raw::badger_ffi_iter_reset(self.handle, &mut err) };
        status_result(status, err)
    }

    pub fn close(&mut self) -> Result<()> {
        if self.handle == 0 {
            return Ok(());
        }

        let mut err = ptr::null_mut();
        let status = unsafe { raw::badger_ffi_iter_close(self.handle, &mut err) };
        status_result(status, err)?;
        self.handle = 0;
        Ok(())
    }
}

impl Drop for BadgerIterator {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn to_c_string(value: &str, field: &str) -> Result<CString> {
    CString::new(value)
        .map_err(|_| BadgerError::InvalidArgument(format!("{field} contains an interior NUL byte")))
}

fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

fn bytes_arg(data: &[u8]) -> (*mut u8, usize) {
    if data.is_empty() {
        (ptr::null_mut(), 0)
    } else {
        (data.as_ptr().cast_mut(), data.len())
    }
}

fn bytes_arg_opt(data: Option<&[u8]>) -> (*mut u8, usize) {
    match data {
        Some(data) => bytes_arg(data),
        None => (ptr::null_mut(), 0),
    }
}

fn take_bytes(ptr: *mut u8, len: usize) -> Option<Vec<u8>> {
    if ptr.is_null() || len == 0 {
        return None;
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr.cast_const(), len) }.to_vec();
    unsafe { raw::badger_ffi_free(ptr.cast::<c_void>()) };
    Some(bytes)
}

fn take_error(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let value = unsafe { CStr::from_ptr(ptr.cast_const()) }
        .to_string_lossy()
        .into_owned();
    unsafe { raw::badger_ffi_free(ptr.cast::<c_void>()) };
    value
}

fn status_result(status: i32, err_ptr: *mut c_char) -> Result<()> {
    let status: Status = status.into();
    if status == Status::Ok {
        return Ok(());
    }

    let err = status.into();
    if matches!(err, BadgerError::InvalidArgument(_)) {
        return Err(BadgerError::InvalidArgument(take_error(err_ptr)));
    }
    Err(err)
}

mod raw {
    use std::ffi::{c_char, c_void};

    #[link(name = "badgerffi")]
    unsafe extern "C" {
        pub fn badger_ffi_free(ptr: *mut c_void);

        pub fn badger_ffi_db_open(
            dir: *const c_char,
            value_dir: *const c_char,
            in_memory: u8,
            encryption_key: *mut u8,
            encryption_key_len: usize,
            index_cache_size: i64,
            value_log_file_size: i64,
            out_handle: *mut u64,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_db_close(handle: u64, err_out: *mut *mut c_char) -> i32;
        pub fn badger_ffi_db_drop_all(handle: u64, err_out: *mut *mut c_char) -> i32;
        pub fn badger_ffi_db_new_txn(
            handle: u64,
            read_only: u8,
            out_handle: *mut u64,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_txn_get(
            handle: u64,
            key: *mut u8,
            key_len: usize,
            out_value: *mut *mut u8,
            out_value_len: *mut usize,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_txn_has(
            handle: u64,
            key: *mut u8,
            key_len: usize,
            out_has: *mut u8,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_txn_set(
            handle: u64,
            key: *mut u8,
            key_len: usize,
            value: *mut u8,
            value_len: usize,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_txn_delete(
            handle: u64,
            key: *mut u8,
            key_len: usize,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_txn_commit(handle: u64, err_out: *mut *mut c_char) -> i32;
        pub fn badger_ffi_txn_discard(handle: u64);

        pub fn badger_ffi_txn_iterator(
            handle: u64,
            prefix: *mut u8,
            prefix_len: usize,
            start: *mut u8,
            start_len: usize,
            end: *mut u8,
            end_len: usize,
            reverse: u8,
            keys_only: u8,
            out_handle: *mut u64,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_iter_next(
            handle: u64,
            out_has: *mut u8,
            err_out: *mut *mut c_char,
        ) -> i32;
        pub fn badger_ffi_iter_key(
            handle: u64,
            out_key: *mut *mut u8,
            out_key_len: *mut usize,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_iter_value(
            handle: u64,
            out_value: *mut *mut u8,
            out_value_len: *mut usize,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_iter_seek(
            handle: u64,
            key: *mut u8,
            key_len: usize,
            out_has: *mut u8,
            err_out: *mut *mut c_char,
        ) -> i32;

        pub fn badger_ffi_iter_reset(handle: u64, err_out: *mut *mut c_char) -> i32;
        pub fn badger_ffi_iter_close(handle: u64, err_out: *mut *mut c_char) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::{Database, IteratorOptions, OpenOptions, Status};

    fn open_in_memory() -> Database {
        Database::open(
            "",
            &OpenOptions {
                in_memory: true,
                ..OpenOptions::default()
            },
        )
        .expect("open in-memory db")
    }

    #[test]
    fn txn_round_trips_none_values() {
        let db = open_in_memory();
        let writer = db.new_txn(false).expect("create write txn");
        writer.set(b"k", None).expect("set nil value");
        writer.commit().expect("commit write txn");

        let reader = db.new_txn(true).expect("create read txn");
        assert!(reader.has(b"k").expect("check key presence"));
        assert_eq!(reader.get(b"k").expect("get key"), None);
    }

    #[test]
    fn iterator_respects_prefix_and_order() {
        let db = open_in_memory();
        let writer = db.new_txn(false).expect("create write txn");
        writer.set(b"aa/1", Some(b"one")).expect("set first value");
        writer.set(b"aa/2", Some(b"two")).expect("set second value");
        writer
            .set(b"bb/1", Some(b"three"))
            .expect("set third value");
        writer.commit().expect("commit write txn");

        let reader = db.new_txn(true).expect("create read txn");
        let mut iter = reader
            .iterator(&IteratorOptions {
                prefix: Some(b"aa/".to_vec()),
                ..IteratorOptions::default()
            })
            .expect("create iterator");

        assert!(iter.has_next().expect("first iterator step"));
        assert_eq!(iter.key().expect("first key"), b"aa/1");
        assert_eq!(iter.value().expect("first value"), Some(b"one".to_vec()));

        assert!(iter.has_next().expect("second iterator step"));
        assert_eq!(iter.key().expect("second key"), b"aa/2");
        assert_eq!(iter.value().expect("second value"), Some(b"two".to_vec()));

        assert!(!iter.has_next().expect("iterator exhaustion"));

        let missing: Status = reader
            .get(b"missing")
            .expect_err("missing key should fail")
            .into();
        assert_eq!(missing, Status::NotFound);
    }
}
