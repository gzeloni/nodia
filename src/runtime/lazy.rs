// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Lazy iterable helpers used by streaming IO and sequence transforms.

use super::*;
use crate::textcodec;
use crate::value::{LazyKind, LazyValue};

impl Runtime {
    pub(super) fn execute_lazy_for(
        &mut self,
        binding: &ForBinding,
        lazy: &LazyValue,
        body: &[Stmt],
    ) -> NodiaResult<Flow> {
        loop {
            let Some(values) = self.next_lazy_bindings(binding, lazy)? else {
                return Ok(Flow::None);
            };
            self.scopes.push(HashMap::new());
            for (name, value) in values {
                self.define(&name, value, true)?;
            }
            let flow = self.execute_block(body)?;
            self.scopes.pop();
            match flow {
                Flow::None | Flow::Continue => {}
                Flow::Break => return Ok(Flow::None),
                Flow::Return(value) => return Ok(Flow::Return(value)),
            }
        }
    }

    pub(super) fn collect_iterable_values(&mut self, iterable: Value) -> NodiaResult<Vec<Value>> {
        match iterable {
            Value::Lazy(lazy) => {
                let mut values = Vec::new();
                while let Some(value) = self.next_lazy_value(&lazy)? {
                    values.push(value);
                }
                Ok(values)
            }
            other => self.iterable_single_values(other),
        }
    }

    fn next_lazy_bindings(
        &mut self,
        binding: &ForBinding,
        lazy: &LazyValue,
    ) -> NodiaResult<Option<Vec<(String, Value)>>> {
        match binding {
            ForBinding::Single(name) => match self.next_lazy_value(lazy)? {
                Some(value) => Ok(Some(vec![(name.clone(), value)])),
                None => Ok(None),
            },
            ForBinding::Pair { key, value } => match self.next_lazy_value(lazy)? {
                Some(item) => {
                    let (left, right) = self.destructure_pair(item)?;
                    Ok(Some(vec![(key.clone(), left), (value.clone(), right)]))
                }
                None => Ok(None),
            },
        }
    }

    pub(super) fn next_lazy_value(&mut self, lazy: &LazyValue) -> NodiaResult<Option<Value>> {
        let snapshot = lazy.snapshot();
        if snapshot.finished {
            return Ok(None);
        }

        match snapshot.kind {
            LazyKind::Lines { stream } => match self.read_line_stream(stream)? {
                Some(line) => Ok(Some(Value::String(line))),
                None => {
                    lazy.finish();
                    Ok(None)
                }
            },
            LazyKind::TextChunks { stream, size } => {
                let chunk = self.read_chunk_stream(stream, size)?;
                if chunk.is_empty() {
                    lazy.finish();
                    Ok(None)
                } else {
                    Ok(Some(Value::String(chunk)))
                }
            }
            LazyKind::ByteChunks { stream, size } => {
                let chunk = self.read_chunk_bytes_stream(stream, size)?;
                if chunk.is_empty() {
                    lazy.finish();
                    Ok(None)
                } else {
                    Ok(Some(textcodec::bytes_to_value(chunk)))
                }
            }
            LazyKind::Map { source, function } => match self.next_lazy_value(&source)? {
                Some(value) => self.invoke_callable1(function, value).map(Some),
                None => {
                    lazy.finish();
                    Ok(None)
                }
            },
            LazyKind::Filter { source, function } => loop {
                match self.next_lazy_value(&source)? {
                    Some(value) => {
                        if self
                            .invoke_callable1(function.clone(), value.clone())?
                            .truthy()
                        {
                            return Ok(Some(value));
                        }
                    }
                    None => {
                        lazy.finish();
                        return Ok(None);
                    }
                }
            },
        }
    }
}
