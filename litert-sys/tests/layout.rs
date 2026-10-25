//! Layout of the ABI-stable structs whose layout differs between compilers.
//! `litert_layout.h` asserts these sizes and offsets in C++; the bindings of a
//! target generated on another host's ABI break them silently, as the Windows
//! MSVC bindings once seeded from macOS put `dimensions` at offset 4.

use std::mem::{size_of, MaybeUninit};
use std::ptr::addr_of;

#[test]
fn litert_layout_matches_the_header() {
    let layout = MaybeUninit::<litert_sys::LiteRtLayout>::uninit();
    let base = layout.as_ptr();
    let offset = |field: *const u8| field as usize - base as usize;
    let dimensions = offset(unsafe { addr_of!((*base).dimensions) }.cast());
    let strides = offset(unsafe { addr_of!((*base).strides) }.cast());

    // MSVC keeps `unsigned int rank : 7` and `bool has_strides : 1` in two
    // allocation units of their own; GCC and Clang pack both into one.
    let (size, expected_dimensions, expected_strides) = if cfg!(target_env = "msvc") {
        (72, 8, 40)
    } else {
        (68, 4, 36)
    };
    assert_eq!(size_of::<litert_sys::LiteRtLayout>(), size, "size");
    assert_eq!(dimensions, expected_dimensions, "offset of dimensions");
    assert_eq!(strides, expected_strides, "offset of strides");
}
