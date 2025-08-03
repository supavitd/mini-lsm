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

#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::sync::Arc;

use crate::key::{KeySlice, KeyVec};

use super::Block;

/// Iterates on a block.
pub struct BlockIterator {
    /// The internal `Block`, wrapped by an `Arc`
    block: Arc<Block>,
    /// The current key, empty represents the iterator is invalid
    key: KeyVec,
    /// the current value range in the block.data, corresponds to the current key
    value_range: (usize, usize),
    /// Current index of the key-value pair, should be in range of [0, num_of_elements)
    idx: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockIterator {
    fn new(block: Arc<Block>) -> Self {
        Self {
            block,
            key: KeyVec::new(),
            value_range: (0, 0),
            idx: 0,
            first_key: KeyVec::new(),
        }
    }

    /// Creates a block iterator and seek to the first entry.
    pub fn create_and_seek_to_first(block: Arc<Block>) -> Self {
        // let key_size = u16::from_le_bytes([block.data[0], block.data[1]]) as usize;
        // let key = block.data[2..key_size].to_owned();
        //
        // let value_size =
        //     u16::from_le_bytes([block.data[key_size], block.data[key_size + 1]]) as usize;
        // let value_start_offset =
        //     std::mem::size_of::<u16>() + key.len() + std::mem::size_of::<u16>();
        // let value_end_offset = value_start_offset + value_size;
        //
        // assert_eq!(value_end_offset, *block.offsets.first().unwrap() as usize);
        //
        // let mut iter = Self::new(block);
        // iter.key.append(&key);
        // iter.first_key.append(&key);
        // iter.value_range = (value_start_offset, value_end_offset);
        //
        let mut iter = Self::new(block);
        iter.seek_to_first();

        iter
    }

    /// Creates a block iterator and seek to the first key that >= `key`.
    pub fn create_and_seek_to_key(block: Arc<Block>, key: KeySlice) -> Self {
        let mut iter = Self::create_and_seek_to_first(block);
        iter.seek_to_key(key);

        iter
    }

    /// Returns the key of the current entry.
    pub fn key(&self) -> KeySlice {
        self.key.as_key_slice()
    }

    /// Returns the value of the current entry.
    pub fn value(&self) -> &[u8] {
        &self.block.data[self.value_range.0..self.value_range.1]
    }

    /// Returns true if the iterator is valid.
    /// Note: You may want to make use of `key`
    pub fn is_valid(&self) -> bool {
        !self.key.is_empty()
    }

    /// Seeks to the first key in the block.
    pub fn seek_to_first(&mut self) {
        // let key_size = u16::from_le_bytes([self.block.data[0], self.block.data[1]]) as usize;
        // let key = self.block.data[2..key_size].to_owned();
        //
        // let value_size =
        //     u16::from_le_bytes([self.block.data[key_size], self.block.data[key_size + 1]]) as usize;
        // let value_start_offset =
        //     std::mem::size_of::<u16>() + key.len() + std::mem::size_of::<u16>();
        // let value_end_offset = value_start_offset + value_size;
        //
        // assert_eq!(
        //     value_end_offset,
        //     *self.block.offsets.first().unwrap() as usize
        // );
        //
        // self.key.append(&key);
        // self.first_key.append(&key);
        // self.value_range = (value_start_offset, value_end_offset);
        self.seek_idx(0);
        self.first_key.append(self.key.raw_ref());
    }

    /// Move to the next key in the block.
    pub fn next(&mut self) {
        let next_idx = self.idx + 1;
        if next_idx >= self.block.offsets.len() {
            self.key.clear();
            return;
        }
        self.seek_idx(next_idx);
    }

    /// Seek to the first key that >= `key`.
    /// Note: You should assume the key-value pairs in the block are sorted when being added by
    /// callers.
    pub fn seek_to_key(&mut self, key: KeySlice) {
        if !self.is_valid() || self.key() > key {
            self.seek_to_first();
        }

        while self.is_valid() && self.key() < key {
            self.next();
        }
    }

    fn seek_idx(&mut self, idx: usize) {
        assert!(idx < self.block.offsets.len(), "Seek index out of bounds");

        let len_size = std::mem::size_of::<u16>();
        let offset = self.block.offsets[idx] as usize;
        // Read key
        let key_len =
            u16::from_le_bytes([self.block.data[offset], self.block.data[offset + 1]]) as usize;
        let key = KeyVec::from_vec(
            self.block.data[offset + len_size..offset + len_size + key_len].to_owned(),
        );

        let value_offset = offset + len_size + key_len;
        let value_len = u16::from_le_bytes([
            self.block.data[value_offset],
            self.block.data[value_offset + 1],
        ]) as usize;

        self.key = if idx == 0 {
            key
        } else {
            let key_raw = key.raw_ref();
            println!("Decompressing key length {}", key_raw.len());
            let key_overlap = u16::from_le_bytes([key_raw[0], key_raw[1]]);
            println!("Key overlap {}", key_overlap);
            let rest_key = &key_raw[4..];
            println!("Rest Key Len {}", rest_key.len());

            let prefix = &self.first_key.raw_ref()[..key_overlap as usize];

            let mut uncompressed_key = KeyVec::new();
            uncompressed_key.append(prefix);
            uncompressed_key.append(rest_key);

            uncompressed_key
        };
        self.idx = idx;
        self.value_range = (value_offset + len_size, value_offset + len_size + value_len);
    }
}
