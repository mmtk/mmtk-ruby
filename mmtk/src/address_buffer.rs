use mmtk::util::Address;

/// This allows the C part to deliver an array of pointers at a time.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AddressBuffer {
    pub ptr: *mut Address,
    pub length: usize,
    pub capacity: usize,
}

impl AddressBuffer {
    pub const DEFAULT_CAPACITY: usize = 512;

    pub fn create() -> Self {
        Self::from(Vec::with_capacity(Self::DEFAULT_CAPACITY))
    }
}

impl From<Vec<Address>> for AddressBuffer {
    fn from(vector: Vec<Address>) -> Self {
        let (ptr, length, capacity )= vector.into_raw_parts();
        AddressBuffer { ptr, length, capacity }
    }
}

impl From<AddressBuffer> for Vec<Address> {
    fn from(buf: AddressBuffer) -> Self {
        unsafe { Vec::from_raw_parts(buf.ptr, buf.length, buf.capacity) }
    }
}
