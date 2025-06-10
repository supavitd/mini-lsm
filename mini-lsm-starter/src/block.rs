// Copyright (c) 2022-2025 Alex Chi Z
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
mod builder;
mod iterator;

pub use builder::BlockBuilder;
use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use iterator::BlockIterator;

/// A block is the smallest unit of read and caching in LSM tree. It is a collection of sorted key-value pairs.
pub struct Block {
    pub(crate) data: Vec<u8>,
    pub(crate) offsets: Vec<u16>,
}

impl Block {
    /// Encode the internal data to the data layout illustrated in the course
    /// Note: You may want to recheck if any of the expected field is missing from your output
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(self.size());

        // TODO: Validate inputs?
        buf.put_slice(&self.data);
        for offset in self.offsets.iter() {
            buf.put_u16_le(*offset);
        }

        let num_of_elements = u16::try_from(self.offsets.len()).unwrap();
        buf.put_u16_le(num_of_elements);

        buf.freeze()
    }

    /// Decode from the data layout, transform the input `data` to a single `Block`
    pub fn decode(data: &[u8]) -> Self {
        let (head, tail) = data.split_at(data.len() - std::mem::size_of::<u16>());
        let mut buf = Bytes::copy_from_slice(head);
        let num_of_elements = u16::from_le_bytes([tail[0], tail[1]]);

        let mut data: Vec<u8> = Vec::with_capacity(num_of_elements as usize);
        let mut offsets: Vec<u16> = Vec::with_capacity(num_of_elements as usize);

        let mut current_entry_offset: u16 = 0;
        let buf_len = buf.len();

        for _ in 0..num_of_elements {
            let key_len = buf.get_u16_le();
            let key = buf.copy_to_bytes(key_len as usize);
            let value_len = buf.get_u16_le();
            let value = buf.copy_to_bytes(value_len as usize);

            data.extend_from_slice(&key_len.to_le_bytes());
            data.extend_from_slice(&key);
            data.extend_from_slice(&value_len.to_le_bytes());
            data.extend_from_slice(&value);

            offsets.push(current_entry_offset);

            let current_pos = buf_len - buf.remaining();
            current_entry_offset =
                u16::try_from(current_pos).expect("u16 overflow from block offset decoding");
        }

        // Validate offsets against incoming
        for (i, offset) in offsets.iter().enumerate() {
            let decoded_offset = buf.get_u16_le();
            assert_eq!(
                decoded_offset, *offset,
                "Index {} Decoded offset {} vs calculated offset {}",
                i, decoded_offset, *offset
            );
        }

        Self { data, offsets }
    }

    fn size(&self) -> usize {
        // Data + Offset + Num of elements
        self.data.len()
            + (self.offsets.len() * std::mem::size_of::<u16>())
            + std::mem::size_of::<u16>()
    }
}
