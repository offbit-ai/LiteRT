//! Introspection for a model's named entry points (signatures).

use std::ptr::NonNull;

use litert_sys as sys;

use crate::{check, Error, Model, Result, TensorShape};

/// A named entry point of a [`Model`]. Most models define a single default
/// signature, exposed as index `0`.
///
/// Holds a clone of the source [`Model`] to keep the underlying C object
/// alive while the signature handle is used.
#[derive(Clone)]
pub struct Signature {
    ptr: NonNull<sys::LiteRtSignatureT>,
    _model: Model,
}

// Safety: LiteRtSignature is a non-owning pointer into an immutable model;
// concurrent read access is safe.
unsafe impl Send for Signature {}
unsafe impl Sync for Signature {}

impl Signature {
    pub(crate) fn new(model: Model, index: sys::LiteRtParamIndex) -> Result<Self> {
        let mut raw: sys::LiteRtSignature = std::ptr::null_mut();
        check(unsafe { sys::LiteRtGetModelSignature(model.as_raw(), index, &mut raw) })?;
        let ptr = NonNull::new(raw).ok_or(Error::NullPointer)?;
        Ok(Self { ptr, _model: model })
    }

    /// Number of input tensors.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) on runtime failure.
    pub fn input_count(&self) -> Result<usize> {
        let mut n: sys::LiteRtParamIndex = 0;
        check(unsafe { sys::LiteRtGetNumSignatureInputs(self.ptr.as_ptr(), &mut n) })?;
        Ok(n)
    }

    /// Number of output tensors.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) on runtime failure.
    pub fn output_count(&self) -> Result<usize> {
        let mut n: sys::LiteRtParamIndex = 0;
        check(unsafe { sys::LiteRtGetNumSignatureOutputs(self.ptr.as_ptr(), &mut n) })?;
        Ok(n)
    }

    /// Shape and element type of the `i`-th input tensor.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Status`](crate::Error::Status) if the index is out of
    /// bounds or the C call fails.
    pub fn input_shape(&self, i: usize) -> Result<TensorShape> {
        let tensor = self.input_tensor(i)?;
        ranked_tensor_shape(tensor)
    }

    /// Shape and element type of the `i`-th output tensor.
    ///
    /// # Errors
    ///
    /// See [`Self::input_shape`].
    pub fn output_shape(&self, i: usize) -> Result<TensorShape> {
        let tensor = self.output_tensor(i)?;
        ranked_tensor_shape(tensor)
    }

    fn input_tensor(&self, i: usize) -> Result<sys::LiteRtTensor> {
        let mut t: sys::LiteRtTensor = std::ptr::null_mut();
        check(unsafe { sys::LiteRtGetSignatureInputTensorByIndex(self.ptr.as_ptr(), i, &mut t) })?;
        if t.is_null() {
            return Err(Error::NullPointer);
        }
        Ok(t)
    }

    fn output_tensor(&self, i: usize) -> Result<sys::LiteRtTensor> {
        let mut t: sys::LiteRtTensor = std::ptr::null_mut();
        check(unsafe { sys::LiteRtGetSignatureOutputTensorByIndex(self.ptr.as_ptr(), i, &mut t) })?;
        if t.is_null() {
            return Err(Error::NullPointer);
        }
        Ok(t)
    }
}

fn ranked_tensor_shape(tensor: sys::LiteRtTensor) -> Result<TensorShape> {
    let mut ranked = sys::LiteRtRankedTensorType::default();
    check(unsafe { sys::LiteRtGetRankedTensorType(tensor, &mut ranked) })?;
    let rank = ranked.layout.rank() as usize;
    Ok(TensorShape {
        element_type: crate::ElementType::from_raw(ranked.element_type),
        dims: ranked.layout.dimensions[..rank].to_vec(),
    })
}
