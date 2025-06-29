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

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use bytes::{Buf, Bytes};

use super::{BlockMeta, FileObject, SsTable};
use crate::{
    block::{Block, BlockBuilder, BlockIterator},
    key::{KeyBytes, KeySlice},
    lsm_storage::BlockCache,
};

/// Builds an SSTable from key-value pairs.
pub struct SsTableBuilder {
    builder: BlockBuilder,
    first_key: Vec<u8>,
    last_key: Vec<u8>,
    data: Vec<u8>,
    pub(crate) meta: Vec<BlockMeta>,
    block_size: usize,
}

impl SsTableBuilder {
    /// Create a builder based on target block size.
    pub fn new(block_size: usize) -> Self {
        Self {
            builder: BlockBuilder::new(block_size),
            first_key: Vec::new(),
            last_key: Vec::new(),
            data: Vec::new(),
            meta: Vec::new(),
            block_size,
        }
    }

    /// Adds a key-value pair to SSTable.
    ///
    /// Note: You should split a new block when the current block is full.(`std::mem::replace` may
    /// be helpful here)
    pub fn add(&mut self, key: KeySlice, value: &[u8]) {
        if self.builder.add(key, value) {
            return;
        }

        self.flush_block();
        self.builder.add(key, value);
    }

    /// Get the estimated size of the SSTable.
    ///
    /// Since the data blocks contain much more data than meta blocks, just return the size of data
    /// blocks here.
    pub fn estimated_size(&self) -> usize {
        self.data.len()
    }

    /// Builds the SSTable and writes it to the given path. Use the `FileObject` structure to manipulate the disk objects.
    pub fn build(
        mut self,
        id: usize,
        block_cache: Option<Arc<BlockCache>>,
        path: impl AsRef<Path>,
    ) -> Result<SsTable> {
        if !self.builder.is_empty() {
            self.flush_block();
        }

        let block_meta_offset = self.data.len();

        let mut meta_buf: Vec<u8> = Vec::new();
        // dbg!("Block meta {}", &self.meta);
        BlockMeta::encode_block_meta(&self.meta, &mut meta_buf);

        let mut data = self.data;
        data.append(&mut meta_buf);
        data.extend_from_slice(
            &u32::try_from(block_meta_offset)
                .expect("Block meta offset must be u32.")
                .to_le_bytes(),
        );

        let file_obj = FileObject::create(path.as_ref(), data)?;

        let sst = SsTable {
            file: file_obj,
            block_meta: self.meta,
            block_meta_offset,
            id,
            block_cache,
            bloom: None,
            first_key: KeyBytes::from_bytes(Bytes::copy_from_slice(&self.first_key)),
            last_key: KeyBytes::from_bytes(Bytes::copy_from_slice(&self.last_key)),
            max_ts: 0,
        };

        Ok(sst)
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(self, path: impl AsRef<Path>) -> Result<SsTable> {
        self.build(0, None, path)
    }

    fn flush_block(&mut self) {
        let prev_block_builder =
            std::mem::replace(&mut self.builder, BlockBuilder::new(self.block_size));
        let block = prev_block_builder.build();
        let block_bytes = block.encode();

        let mut block_iter = BlockIterator::create_and_seek_to_first(Arc::new(block));

        let block_first_key =
            KeyBytes::from_bytes(Bytes::copy_from_slice(block_iter.key().raw_ref()));
        let block_last_key = loop {
            // TODO: Can this be improved so that we don't need to copy every iteration?
            let block_key_bytes =
                KeyBytes::from_bytes(Bytes::copy_from_slice(block_iter.key().raw_ref()));
            block_iter.next();
            if !block_iter.is_valid() {
                break block_key_bytes;
            }
        };

        let block_meta = BlockMeta {
            offset: self.data.len(),
            first_key: block_first_key,
            last_key: block_last_key,
        };

        if self.first_key.is_empty() {
            self.first_key
                .extend_from_slice(block_meta.first_key.raw_ref());
        }

        self.last_key = block_meta.last_key.raw_ref().to_vec();

        self.data.extend_from_slice(&block_bytes);
        self.meta.push(block_meta);
    }
}
