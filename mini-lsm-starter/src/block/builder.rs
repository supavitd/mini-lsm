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

use bytes::BufMut;

use crate::key::{Key, KeySlice, KeyVec};

use super::Block;

/// Builds a block.
pub struct BlockBuilder {
    /// Offsets of each key-value entries.
    offsets: Vec<u16>,
    /// All serialized key-value pairs in the block.
    data: Vec<u8>,
    /// The expected block size.
    block_size: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockBuilder {
    const KEY_LEN_BYTE: usize = std::mem::size_of::<u16>();
    const VALUE_LEN_BYTE: usize = std::mem::size_of::<u16>();

    /// Creates a new block builder.
    pub fn new(block_size: usize) -> Self {
        Self {
            offsets: Vec::new(),
            data: Vec::new(),
            block_size,
            first_key: Key::new(),
        }
    }

    /// Adds a key-value pair to the block. Returns false when the block is full.
    /// You may find the `bytes::BufMut` trait useful for manipulating binary data.
    #[must_use]
    pub fn add(&mut self, key: KeySlice, value: &[u8]) -> bool {
        let compressed_key = if self.first_key.is_empty() {
            key.raw_ref()
        } else {
            &self.compress_key(key)
        };

        let entry_size =
            compressed_key.len() + value.len() + Self::KEY_LEN_BYTE + Self::VALUE_LEN_BYTE;

        let current_block_size =
            self.data.len() + (self.offsets.len() * std::mem::size_of::<u16>());

        if !self.first_key.is_empty() && current_block_size + entry_size > self.block_size {
            println!(
                "Block is full. {} {}",
                current_block_size + entry_size,
                self.block_size
            );
            return false;
        }

        if self.first_key.is_empty() {
            self.first_key.append(key.raw_ref());
        }

        // Offset block
        let offset = u16::try_from(self.data.len()).unwrap();
        self.offsets.push(offset);

        // Entry block
        self.data
            .put_u16_le(compressed_key.len().try_into().unwrap());
        self.data.put_slice(compressed_key);
        self.data.put_u16_le(value.len().try_into().unwrap());
        self.data.put_slice(value);

        true
    }

    /// Check if there is no key-value pair in the block.
    pub fn is_empty(&self) -> bool {
        self.first_key.is_empty()
    }

    /// Finalize the block.
    pub fn build(self) -> Block {
        Block {
            data: self.data,
            offsets: self.offsets,
        }
    }

    fn compress_key(&self, key: KeySlice) -> Vec<u8> {
        assert!(
            !self.first_key.is_empty(),
            "Cannot compress key since first key is not set."
        );
        let mut key_overlap: u16 = 0;
        let min_size = self.first_key.len().min(key.len());

        let first_key_raw = self.first_key.raw_ref();
        let key_raw = key.raw_ref();
        for i in 0..min_size {
            if first_key_raw[i] == key_raw[i] {
                key_overlap += 1;
            } else {
                break;
            }
        }

        let key_rest =
            u16::try_from(key.len() - key_overlap as usize).expect("Rest key len must be u16.");

        let mut compressed = Vec::<u8>::new();
        compressed.put_u16_le(key_overlap);
        compressed.put_u16_le(key_rest);
        compressed.extend_from_slice(&key_raw[key_overlap as usize..]);

        compressed
    }
}
