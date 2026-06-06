// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! File and stream helpers used by the runtime I/O built-ins.

use crate::error::{NodiaError, NodiaResult};
use crate::value::StreamId;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, Write};

/// Registry of open file-backed streams managed by the runtime.
pub struct IoRegistry {
    next_file_id: usize,
    streams: HashMap<usize, FileStream>,
}

impl Default for IoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

enum FileStream {
    Reader {
        reader: BufReader<File>,
        eof: bool,
        byte_buffer: Vec<u8>,
    },
    Writer {
        writer: BufWriter<File>,
    },
}

impl IoRegistry {
    /// Creates an empty stream registry.
    pub fn new() -> Self {
        Self {
            next_file_id: 1,
            streams: HashMap::new(),
        }
    }

    /// Opens a file stream in `read`, `write`, or `append` mode.
    pub fn open(&mut self, path: &str, mode: &str, allow_write: bool) -> NodiaResult<StreamId> {
        let stream = match mode {
            "read" => FileStream::Reader {
                reader: BufReader::new(File::open(path).map_err(|err| {
                    NodiaError::io(format!("cannot open '{path}' for read: {err}"))
                })?),
                eof: false,
                byte_buffer: Vec::new(),
            },
            "write" => {
                ensure_write_allowed(allow_write)?;
                FileStream::Writer {
                    writer: BufWriter::new(File::create(path).map_err(|err| {
                        NodiaError::io(format!("cannot open '{path}' for write: {err}"))
                    })?),
                }
            }
            "append" => {
                ensure_write_allowed(allow_write)?;
                FileStream::Writer {
                    writer: BufWriter::new(
                        OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .map_err(|err| {
                                NodiaError::io(format!("cannot open '{path}' for append: {err}"))
                            })?,
                    ),
                }
            }
            _ => {
                return Err(NodiaError::runtime(format!(
                    "open() mode must be 'read', 'write' or 'append', got '{mode}'"
                )))
            }
        };

        let id = self.next_file_id;
        self.next_file_id += 1;
        self.streams.insert(id, stream);
        Ok(StreamId::File(id))
    }

    /// Closes an open stream and flushes buffered writers.
    pub fn close(&mut self, stream: StreamId) -> NodiaResult<()> {
        let StreamId::File(id) = stream else {
            return Ok(());
        };
        let Some(mut stream) = self.streams.remove(&id) else {
            return Err(NodiaError::runtime(format!("stream {id} is closed")));
        };
        if let FileStream::Writer { writer } = &mut stream {
            writer.flush().map_err(|err| {
                NodiaError::io(format!("cannot flush stream {id} before close: {err}"))
            })?;
        }
        Ok(())
    }

    /// Flushes a writable stream.
    pub fn flush(&mut self, stream: StreamId) -> NodiaResult<()> {
        let id = expect_file_stream(stream, "flush")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Writer { writer } => writer
                .flush()
                .map_err(|err| NodiaError::io(format!("cannot flush stream {id}: {err}"))),
            FileStream::Reader { .. } => Err(NodiaError::runtime(
                "flush() expects writable stream, got readable stream",
            )),
        }
    }

    /// Reports whether a readable stream has reached end-of-file.
    pub fn eof(&mut self, stream: StreamId) -> NodiaResult<bool> {
        let id = expect_file_stream(stream, "eof")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader { eof, .. } => Ok(*eof),
            FileStream::Writer { .. } => Err(NodiaError::runtime(
                "eof() expects readable stream, got writable stream",
            )),
        }
    }

    /// Reads the remainder of a readable stream as UTF-8 text.
    pub fn read_all(&mut self, stream: StreamId) -> NodiaResult<String> {
        let id = expect_file_stream(stream, "read")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader {
                reader,
                eof,
                byte_buffer,
            } => {
                byte_buffer.clear();
                byte_buffer.reserve(remaining_file_hint(reader));
                reader.read_to_end(byte_buffer).map_err(|err| {
                    NodiaError::io(format!("cannot read from stream {id}: {err}"))
                })?;
                *eof = true;
                let mut bytes = Vec::with_capacity(byte_buffer.capacity());
                std::mem::swap(byte_buffer, &mut bytes);
                into_utf8_string(bytes, &format!("cannot read from stream {id}"))
            }
            FileStream::Writer { .. } => Err(NodiaError::runtime(
                "read() expects readable stream, got writable stream",
            )),
        }
    }

    /// Reads up to `size` bytes from a readable stream.
    pub fn read_chunk(&mut self, stream: StreamId, size: usize) -> NodiaResult<String> {
        let id = expect_file_stream(stream, "read")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader {
                reader,
                eof,
                byte_buffer,
                ..
            } => {
                if size == 0 {
                    return Ok(String::new());
                }

                byte_buffer.clear();
                loop {
                    match std::str::from_utf8(byte_buffer) {
                        Ok(text) if byte_buffer.len() >= size => return Ok(text.to_string()),
                        Ok(_) => {}
                        Err(err) if err.error_len().is_some() => {
                            let mut bytes = Vec::with_capacity(byte_buffer.capacity());
                            std::mem::swap(byte_buffer, &mut bytes);
                            return into_utf8_string(
                                bytes,
                                &format!("cannot read from stream {id}"),
                            );
                        }
                        Err(_) => {}
                    }

                    let start = byte_buffer.len();
                    let read_len = if start < size { size - start } else { 1 };
                    byte_buffer.resize(start + read_len, 0);
                    let read = reader.read(&mut byte_buffer[start..]).map_err(|err| {
                        NodiaError::io(format!("cannot read from stream {id}: {err}"))
                    })?;
                    if read == 0 {
                        *eof = true;
                        byte_buffer.truncate(start);
                        if byte_buffer.is_empty() {
                            return Ok(String::new());
                        }
                        let mut bytes = Vec::with_capacity(byte_buffer.capacity());
                        std::mem::swap(byte_buffer, &mut bytes);
                        return into_utf8_string(bytes, &format!("cannot read from stream {id}"));
                    }
                    byte_buffer.truncate(start + read);
                }
            }
            FileStream::Writer { .. } => Err(NodiaError::runtime(
                "read() expects readable stream, got writable stream",
            )),
        }
    }

    /// Reads a single logical line without the trailing newline sequence.
    pub fn read_line(&mut self, stream: StreamId) -> NodiaResult<Option<String>> {
        let id = expect_file_stream(stream, "readln")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader {
                reader,
                eof,
                byte_buffer,
                ..
            } => {
                byte_buffer.clear();
                let read = reader.read_until(b'\n', byte_buffer).map_err(|err| {
                    NodiaError::io(format!("cannot read line from stream {id}: {err}"))
                })?;
                if read == 0 {
                    *eof = true;
                    return Ok(None);
                }
                if byte_buffer.ends_with(b"\n") {
                    byte_buffer.pop();
                    if byte_buffer.ends_with(b"\r") {
                        byte_buffer.pop();
                    }
                }
                let mut bytes = Vec::with_capacity(byte_buffer.capacity());
                std::mem::swap(byte_buffer, &mut bytes);
                into_utf8_string(bytes, &format!("cannot read line from stream {id}")).map(Some)
            }
            FileStream::Writer { .. } => Err(NodiaError::runtime(
                "readln() expects readable stream, got writable stream",
            )),
        }
    }

    /// Writes text to a writable stream without appending a newline.
    pub fn write(&mut self, stream: StreamId, text: &str) -> NodiaResult<()> {
        let id = expect_file_stream(stream, "write")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Writer { writer } => writer
                .write_all(text.as_bytes())
                .map_err(|err| NodiaError::io(format!("cannot write to stream {id}: {err}"))),
            FileStream::Reader { .. } => Err(NodiaError::runtime(
                "write() expects writable stream, got readable stream",
            )),
        }
    }

    /// Flushes every tracked writable stream.
    pub fn flush_all(&mut self) -> NodiaResult<()> {
        for (id, stream) in &mut self.streams {
            if let FileStream::Writer { writer } = stream {
                writer
                    .flush()
                    .map_err(|err| NodiaError::io(format!("cannot flush stream {id}: {err}")))?;
            }
        }
        Ok(())
    }

    fn stream_mut(&mut self, id: usize) -> NodiaResult<&mut FileStream> {
        self.streams
            .get_mut(&id)
            .ok_or_else(|| NodiaError::runtime(format!("stream {id} is closed")))
    }
}

/// Reads an entire file path into memory.
pub fn read_path(path: &str) -> NodiaResult<String> {
    let mut file =
        File::open(path).map_err(|err| NodiaError::io(format!("cannot read '{path}': {err}")))?;
    let mut bytes = Vec::with_capacity(file_size_hint(&file));
    file.read_to_end(&mut bytes)
        .map_err(|err| NodiaError::io(format!("cannot read '{path}': {err}")))?;
    into_utf8_string(bytes, &format!("cannot read '{path}'"))
}

/// Writes a full string to a file path, replacing existing content.
pub fn write_path(path: &str, text: &str, allow_write: bool) -> NodiaResult<()> {
    ensure_write_allowed(allow_write)?;
    fs::write(path, text).map_err(|err| NodiaError::io(format!("cannot write '{path}': {err}")))
}

/// Appends a string to a file path.
pub fn append_path(path: &str, text: &str, allow_write: bool) -> NodiaResult<()> {
    ensure_write_allowed(allow_write)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| NodiaError::io(format!("cannot append '{path}': {err}")))?;
    file.write_all(text.as_bytes())
        .map_err(|err| NodiaError::io(format!("cannot append '{path}': {err}")))
}

/// Fails when filesystem writes are not enabled in runtime options.
pub fn ensure_write_allowed(allow_write: bool) -> NodiaResult<()> {
    if allow_write {
        Ok(())
    } else {
        Err(NodiaError::io("file write requires --allow-write").with_code("E3001"))
    }
}

fn expect_file_stream(stream: StreamId, name: &str) -> NodiaResult<usize> {
    match stream {
        StreamId::File(id) => Ok(id),
        StreamId::Stdin | StreamId::Stdout | StreamId::Stderr => Err(NodiaError::runtime(format!(
            "{name}() cannot use {stream} through the file registry"
        ))),
    }
}

fn file_size_hint(file: &File) -> usize {
    file.metadata()
        .ok()
        .map(|meta| meta.len().min(usize::MAX as u64) as usize)
        .unwrap_or(0)
}

fn remaining_file_hint(reader: &mut BufReader<File>) -> usize {
    let Ok(position) = reader.stream_position() else {
        return 0;
    };
    reader
        .get_ref()
        .metadata()
        .ok()
        .map(|meta| meta.len().saturating_sub(position).min(usize::MAX as u64) as usize)
        .unwrap_or(0)
}

fn into_utf8_string(bytes: Vec<u8>, context: &str) -> NodiaResult<String> {
    String::from_utf8(bytes).map_err(|err| NodiaError::io(format!("{context}: {err}")))
}
