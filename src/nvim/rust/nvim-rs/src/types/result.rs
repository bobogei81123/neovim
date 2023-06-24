use std::{mem::{ManuallyDrop, self}, ffi::{CString, c_char}, fmt::Display};

/// Represents the result of calling neovim functions.
///
/// It wraps nvim's `Error` (see nvim/api/private/defs.h) and resembles `Result<(), NvimError>` in
/// rust. Nvim's `Error` has a "none" state which makes it more like `Result` than an `Error`, in
/// Rust's point of view.
pub struct NvimResult(ManuallyDrop<nvim_sys::Error>);

impl NvimResult {
    /// Creates a new Ok `NvimResult`.
    pub fn new_ok() -> Self {
        Self(ManuallyDrop::new(nvim_sys::Error {
            type_: nvim_sys::ErrorType_kErrorTypeNone,
            msg: std::ptr::null_mut(),
        }))
    }

    /// Consumes this object and returns an Neovim's `Error`.
    ///
    /// The caller is responsible for freeing the returned `Error`.
    pub fn into_ffi(mut self) -> nvim_sys::Error {
        let inner = unsafe { ManuallyDrop::take(&mut self.0) };
        mem::forget(self);
        inner
    }

    /// Returns a borrowed Neovim `Error`.
    ///
    /// The caller must make sure that the object remain valid when the borrow ended.
    pub fn as_borrowed_ffi_mut(&mut self) -> &mut nvim_sys::Error {
        &mut self.0
    }

    /// Creates an NvimResult from Rust's [`Result`](std::result::Result) type.
    pub fn from_result(result: std::result::Result<(), NvimError>) -> Self {
        let (type_, msg) = match result {
            Ok(()) => (nvim_sys::ErrorType_kErrorTypeNone, std::ptr::null_mut()),
            Err(err) => {
                let type_ = match err.kind {
                    NvimErrorKind::Exception => nvim_sys::ErrorType_kErrorTypeException,
                    NvimErrorKind::Validation => nvim_sys::ErrorType_kErrorTypeValidation,
                };
                let msg = err.msg.as_ptr() as *mut i8;
                (type_, msg)
            }
        };
        Self(ManuallyDrop::new(nvim_sys::Error { type_, msg }))
    }

    /// Converts this object into a Rust [`Result`](std::result::Result).
    pub fn into_result(self) -> std::result::Result<(), NvimError> {
        let nvim_sys::Error { type_, msg } = self.into_ffi();
        match type_ {
            nvim_sys::ErrorType_kErrorTypeNone => Ok(()),
            nvim_sys::ErrorType_kErrorTypeException => Err(NvimError {
                kind: NvimErrorKind::Exception,
                msg: unsafe { cstring_from_raw_check_null(msg) },
            }),
            nvim_sys::ErrorType_kErrorTypeValidation => Err(NvimError {
                kind: NvimErrorKind::Validation,
                msg: unsafe { cstring_from_raw_check_null(msg) },
            }),
            _ => {
                panic!(
                    "Encounter unknown error value ({:?}) when converting nvim error",
                    type_
                );
            }
        }
    }
}

impl Drop for NvimResult {
    fn drop(&mut self) {
        unsafe { nvim_sys::xfree_clear(&mut self.0.msg) }
    }
}

impl Default for NvimResult {
    /// Returns the default value, which is an Ok result.
    fn default() -> Self {
        NvimResult::new_ok()
    }
}

impl From<NvimResult> for std::result::Result<(), NvimError> {
    fn from(value: NvimResult) -> Self {
        value.into_result()
    }
}

/// Wraps nvim's `ErrorType` (see nvim/api/private/defs.h).
#[derive(Debug)]
pub enum NvimErrorKind {
    Exception,
    Validation,
}

#[derive(Debug)]
/// The error type when converting `NvimResult` into a Rust [`Result`](std::result::Result).
pub struct NvimError {
    pub kind: NvimErrorKind,
    pub msg: CString,
}

impl Display for NvimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            NvimErrorKind::Exception => "Exception: ",
            NvimErrorKind::Validation => "Validation: ",
        };
        write!(f, "{kind}: {}", self.msg.to_string_lossy())
    }
}

impl std::error::Error for NvimError {}

unsafe fn cstring_from_raw_check_null(msg: *mut c_char) -> CString {
    if msg.is_null() {
        panic!("Try to covert a null pointer to a CString");
    }
    unsafe { CString::from_raw(msg) }
}
