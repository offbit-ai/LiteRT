//! Zero-copy I/O buffers for compiled model inference.

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
};

use litert_sys as sys;

use crate::{check, ElementType, Environment, Error, Result, TensorElement};

/// Shape + element type of a ranked tensor.
#[derive(Debug, Clone)]
pub struct TensorShape {
    /// Element type of each value.
    pub element_type: ElementType,
    /// Dimension sizes, in declaration order. Length is the tensor's rank.
    pub dims: Vec<i32>,
}

impl TensorShape {
    /// Total number of scalar elements (`product of dims`).
    #[must_use]
    pub fn num_elements(&self) -> usize {
        self.dims.iter().map(|&d| d.max(0) as usize).product()
    }

    pub(crate) fn to_raw(&self) -> sys::LiteRtRankedTensorType {
        let mut layout = sys::LiteRtLayout::default();
        layout.set_rank(u32::try_from(self.dims.len()).expect("rank fits in u32"));
        layout.set_has_strides(false);
        for (slot, &d) in layout.dimensions.iter_mut().zip(self.dims.iter()) {
            *slot = d;
        }
        sys::LiteRtRankedTensorType {
            element_type: self.element_type as sys::LiteRtElementType,
            layout,
        }
    }

    fn from_raw(raw: &sys::LiteRtRankedTensorType) -> Self {
        let rank = raw.layout.rank() as usize;
        Self {
            element_type: ElementType::from_raw(raw.element_type),
            dims: raw.layout.dimensions[..rank].to_vec(),
        }
    }
}

/// A memory buffer bound to a compiled tensor.
///
/// Most users obtain one from
/// [`CompiledModel`](crate::CompiledModel)-derived helpers (coming in a later
/// phase) or via [`Self::managed_host`], and only lock it to read/write the
/// underlying data with a strongly typed slice.
pub struct TensorBuffer {
    ptr: NonNull<sys::LiteRtTensorBufferT>,
}

impl std::fmt::Debug for TensorBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TensorBuffer")
            .field("ptr", &self.ptr.as_ptr())
            .finish()
    }
}

impl TensorBuffer {
    /// Allocates a managed host-memory tensor buffer of the given shape.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if allocation fails.
    pub fn managed_host(env: &Environment, shape: &TensorShape) -> Result<Self> {
        let raw_type = shape.to_raw();
        let element_size = element_size_bytes(shape.element_type).unwrap_or(0);
        let size_bytes = shape.num_elements() * element_size;

        let mut raw: sys::LiteRtTensorBuffer = std::ptr::null_mut();
        check(unsafe {
            sys::LiteRtCreateManagedTensorBuffer(
                env.as_raw(),
                sys::kLiteRtTensorBufferTypeHostMemory,
                &raw_type,
                size_bytes,
                &mut raw,
            )
        })?;
        let ptr = NonNull::new(raw).ok_or(Error::NullPointer)?;
        Ok(Self { ptr })
    }

    /// Size of the underlying buffer in bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if the C call fails.
    pub fn size_bytes(&self) -> Result<usize> {
        let mut size = 0usize;
        check(unsafe { sys::LiteRtGetTensorBufferSize(self.ptr.as_ptr(), &mut size) })?;
        Ok(size)
    }

    /// Shape and element type of the bound tensor.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if the C call fails.
    pub fn shape(&self) -> Result<TensorShape> {
        let mut raw = sys::LiteRtRankedTensorType::default();
        check(unsafe { sys::LiteRtGetTensorBufferTensorType(self.ptr.as_ptr(), &mut raw) })?;
        Ok(TensorShape::from_raw(&raw))
    }

    /// Locks the buffer for reading. Returns a guard whose `Deref<Target=[T]>`
    /// impl gives zero-copy access to the data.
    ///
    /// # Errors
    ///
    /// - [`Error::TypeMismatch`] if the tensor's element type differs from `T`.
    /// - [`Error::UnalignedBufferSize`] if the byte size isn't a whole number
    ///   of `T` values.
    /// - [`Error::Status`](crate::Error::Status) on runtime failure.
    pub fn lock_for_read<T: TensorElement>(&self) -> Result<ReadGuard<'_, T>> {
        let (ptr, len) = self.lock::<T>(sys::kLiteRtTensorBufferLockModeRead)?;
        Ok(ReadGuard {
            buffer: self,
            ptr,
            len,
            _phantom: PhantomData,
        })
    }

    /// Locks the buffer for writing.
    ///
    /// # Errors
    ///
    /// See [`Self::lock_for_read`].
    pub fn lock_for_write<T: TensorElement>(&mut self) -> Result<WriteGuard<'_, T>> {
        let (ptr, len) = self.lock::<T>(sys::kLiteRtTensorBufferLockModeWrite)?;
        Ok(WriteGuard {
            buffer: self.ptr,
            ptr,
            len,
            _phantom: PhantomData,
        })
    }

    fn lock<T: TensorElement>(
        &self,
        mode: sys::LiteRtTensorBufferLockMode,
    ) -> Result<(*mut T, usize)> {
        let actual = self.shape()?.element_type;
        if actual != T::TYPE {
            return Err(Error::TypeMismatch {
                expected: T::TYPE,
                actual,
            });
        }
        let size = self.size_bytes()?;
        let element_size = std::mem::size_of::<T>();
        if element_size == 0 || size % element_size != 0 {
            return Err(Error::UnalignedBufferSize {
                size,
                element_size,
                type_name: T::NAME,
            });
        }
        let mut host: *mut std::ffi::c_void = std::ptr::null_mut();
        check(unsafe { sys::LiteRtLockTensorBuffer(self.ptr.as_ptr(), &mut host, mode) })?;
        if host.is_null() {
            return Err(Error::NullPointer);
        }
        Ok((host.cast(), size / element_size))
    }

    pub(crate) fn as_raw(&self) -> sys::LiteRtTensorBuffer {
        self.ptr.as_ptr()
    }
}

impl Drop for TensorBuffer {
    fn drop(&mut self) {
        unsafe { sys::LiteRtDestroyTensorBuffer(self.ptr.as_ptr()) }
    }
}

// Safety: handles are opaque C pointers; the safe API prevents shared mutation.
unsafe impl Send for TensorBuffer {}

/// RAII guard granting shared read access to a locked tensor buffer.
pub struct ReadGuard<'a, T> {
    buffer: &'a TensorBuffer,
    ptr: *mut T,
    len: usize,
    _phantom: PhantomData<&'a [T]>,
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        // Safety: the lock succeeded; the buffer outlives the guard; the
        // slice length is derived from the verified byte size.
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<'a, T> Drop for ReadGuard<'a, T> {
    fn drop(&mut self) {
        // A failed unlock here would leak the lock state inside LiteRT but we
        // cannot surface an error from Drop. Best-effort.
        unsafe { sys::LiteRtUnlockTensorBuffer(self.buffer.ptr.as_ptr()) };
    }
}

/// RAII guard granting exclusive write access to a locked tensor buffer.
pub struct WriteGuard<'a, T> {
    buffer: NonNull<sys::LiteRtTensorBufferT>,
    ptr: *mut T,
    len: usize,
    _phantom: PhantomData<&'a mut [T]>,
}

impl<'a, T> Deref for WriteGuard<'a, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<'a, T> DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { sys::LiteRtUnlockTensorBuffer(self.buffer.as_ptr()) };
    }
}

fn element_size_bytes(et: ElementType) -> Option<usize> {
    Some(match et {
        ElementType::Bool | ElementType::Int8 | ElementType::UInt8 => 1,
        ElementType::Int16 | ElementType::UInt16 | ElementType::Float16 | ElementType::BFloat16 => {
            2
        }
        ElementType::Int32 | ElementType::UInt32 | ElementType::Float32 => 4,
        ElementType::Int64
        | ElementType::UInt64
        | ElementType::Float64
        | ElementType::Complex64 => 8,
        ElementType::Complex128 => 16,
        // Sub-byte or opaque types — let the runtime tell us the exact size.
        ElementType::Int2 | ElementType::Int4 | ElementType::None => return None,
    })
}
