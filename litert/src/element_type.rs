//! Tensor element types and the `TensorElement` trait.

use litert_sys as sys;

/// Scalar element type of a tensor, in one-to-one correspondence with the
/// LiteRT C enum `LiteRtElementType`. Variant names track the Rust primitive
/// types or IEEE/INT naming (`Float16`, `Int32`) where no Rust analogue exists.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
#[non_exhaustive]
pub enum ElementType {
    None = sys::kLiteRtElementTypeNone,
    Bool = sys::kLiteRtElementTypeBool,
    Int2 = sys::kLiteRtElementTypeInt2,
    Int4 = sys::kLiteRtElementTypeInt4,
    Int8 = sys::kLiteRtElementTypeInt8,
    Int16 = sys::kLiteRtElementTypeInt16,
    Int32 = sys::kLiteRtElementTypeInt32,
    Int64 = sys::kLiteRtElementTypeInt64,
    UInt8 = sys::kLiteRtElementTypeUInt8,
    UInt16 = sys::kLiteRtElementTypeUInt16,
    UInt32 = sys::kLiteRtElementTypeUInt32,
    UInt64 = sys::kLiteRtElementTypeUInt64,
    Float16 = sys::kLiteRtElementTypeFloat16,
    BFloat16 = sys::kLiteRtElementTypeBFloat16,
    Float32 = sys::kLiteRtElementTypeFloat32,
    Float64 = sys::kLiteRtElementTypeFloat64,
    Complex64 = sys::kLiteRtElementTypeComplex64,
    Complex128 = sys::kLiteRtElementTypeComplex128,
}

impl ElementType {
    pub(crate) fn from_raw(raw: sys::LiteRtElementType) -> Self {
        match raw {
            sys::kLiteRtElementTypeBool => Self::Bool,
            sys::kLiteRtElementTypeInt2 => Self::Int2,
            sys::kLiteRtElementTypeInt4 => Self::Int4,
            sys::kLiteRtElementTypeInt8 => Self::Int8,
            sys::kLiteRtElementTypeInt16 => Self::Int16,
            sys::kLiteRtElementTypeInt32 => Self::Int32,
            sys::kLiteRtElementTypeInt64 => Self::Int64,
            sys::kLiteRtElementTypeUInt8 => Self::UInt8,
            sys::kLiteRtElementTypeUInt16 => Self::UInt16,
            sys::kLiteRtElementTypeUInt32 => Self::UInt32,
            sys::kLiteRtElementTypeUInt64 => Self::UInt64,
            sys::kLiteRtElementTypeFloat16 => Self::Float16,
            sys::kLiteRtElementTypeBFloat16 => Self::BFloat16,
            sys::kLiteRtElementTypeFloat32 => Self::Float32,
            sys::kLiteRtElementTypeFloat64 => Self::Float64,
            sys::kLiteRtElementTypeComplex64 => Self::Complex64,
            sys::kLiteRtElementTypeComplex128 => Self::Complex128,
            _ => Self::None,
        }
    }
}

/// Marker trait for Rust scalars that correspond to a LiteRT `ElementType`.
///
/// Implemented for `bool`, `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`,
/// `u64`, `f32`, `f64`.
///
/// # Safety
///
/// Implementors promise that `Self` is `Sized`, `Copy`, has no drop glue, and
/// that a `&[Self]` can be safely reinterpreted from a `*mut u8` buffer of
/// `N * size_of::<Self>()` bytes that LiteRT returned as a tensor whose
/// element type equals [`Self::TYPE`].
pub unsafe trait TensorElement: Copy + Sized + 'static {
    /// The `ElementType` corresponding to `Self`.
    const TYPE: ElementType;

    /// Human-readable name, used in error messages.
    const NAME: &'static str;
}

macro_rules! impl_tensor_element {
    ($($rust:ty => ($variant:ident, $name:literal)),* $(,)?) => {
        $(
            unsafe impl TensorElement for $rust {
                const TYPE: ElementType = ElementType::$variant;
                const NAME: &'static str = $name;
            }
        )*
    };
}

impl_tensor_element! {
    bool => (Bool,    "bool"),
    i8   => (Int8,    "i8"),
    i16  => (Int16,   "i16"),
    i32  => (Int32,   "i32"),
    i64  => (Int64,   "i64"),
    u8   => (UInt8,   "u8"),
    u16  => (UInt16,  "u16"),
    u32  => (UInt32,  "u32"),
    u64  => (UInt64,  "u64"),
    f32  => (Float32, "f32"),
    f64  => (Float64, "f64"),
}
