use mmtk::util::Address;
use mmtk::util::ObjectReference;

/// This allows the C part to deliver an array of pointers at a time.
#[repr(C)]
pub struct AddressBuffer {
    pub ptr: *mut Address,
    pub length: usize,
    pub capacity: usize,
}

/// This represents a filled AddressBuffer.
/// The only thing you can do is to interpret it as a Vec<T>,
/// where T can be either Address or ObjectReference.
pub struct FilledBuffer {
    buffer: AddressBuffer,
}

impl AddressBuffer {
    pub const DEFAULT_CAPACITY: usize = 512;

    pub fn create() -> Self {
        let vector = Vec::with_capacity(Self::DEFAULT_CAPACITY);
        let (ptr, length, capacity ) = vector.into_raw_parts();
        AddressBuffer { ptr, length, capacity }
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn take_as_filled_buffer(&mut self) -> FilledBuffer {
        let taken_self = std::mem::replace(self, Self::create());
        FilledBuffer { buffer: taken_self }
    }
}

impl FilledBuffer {
    pub fn as_address_vec(self) -> Vec<Address> {
        unsafe { Vec::from_raw_parts(self.buffer.ptr, self.buffer.length, self.buffer.capacity) }
    }

    pub fn as_objref_vec(self) -> Vec<ObjectReference> {
        unsafe { Vec::from_raw_parts(self.buffer.ptr as *mut ObjectReference, self.buffer.length, self.buffer.capacity) }
    }
}
