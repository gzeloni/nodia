// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! File and stream helpers used by the runtime I/O built-ins.

use crate::error::{NodiaError, NodiaResult};
use crate::textcodec;
use crate::value::StreamId;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, Write};
use std::net::{TcpListener as StdTcpListener, TcpStream};

/// Registry of open file-backed streams managed by the runtime.
pub struct IoRegistry {
    next_file_id: usize,
    streams: HashMap<usize, FileStream>,
    next_tcp_id: usize,
    tcp_streams: HashMap<usize, TcpConnection>,
    tcp_listeners: HashMap<usize, StdTcpListener>,
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

pub(super) struct TcpConnection {
    pub(super) stream: TcpStream,
    pub(super) read_buffer: Vec<u8>,
    pub(super) eof: bool,
}

impl IoRegistry {
    /// Creates an empty stream registry.
    pub fn new() -> Self {
        Self {
            next_file_id: 1,
            streams: HashMap::new(),
            next_tcp_id: 1,
            tcp_streams: HashMap::new(),
            tcp_listeners: HashMap::new(),
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
        let bytes = self.read_all_bytes(stream)?;
        textcodec::decode_utf8_io(bytes, &format!("cannot read from stream {id}"))
    }

    /// Reads the remainder of a readable stream as raw bytes.
    pub fn read_all_bytes(&mut self, stream: StreamId) -> NodiaResult<Vec<u8>> {
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
                Ok(bytes)
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
                            return textcodec::decode_utf8_io(
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
                        return textcodec::decode_utf8_io(
                            bytes,
                            &format!("cannot read from stream {id}"),
                        );
                    }
                    byte_buffer.truncate(start + read);
                }
            }
            FileStream::Writer { .. } => Err(NodiaError::runtime(
                "read() expects readable stream, got writable stream",
            )),
        }
    }

    /// Reads up to `size` bytes from a readable stream without UTF-8 decoding.
    pub fn read_chunk_bytes(&mut self, stream: StreamId, size: usize) -> NodiaResult<Vec<u8>> {
        let id = expect_file_stream(stream, "read_bytes")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader {
                reader,
                eof,
                byte_buffer,
                ..
            } => {
                if size == 0 {
                    return Ok(Vec::new());
                }

                byte_buffer.resize(size, 0);
                let read = reader.read(byte_buffer).map_err(|err| {
                    NodiaError::io(format!("cannot read from stream {id}: {err}"))
                })?;
                if read == 0 {
                    *eof = true;
                    byte_buffer.clear();
                    return Ok(Vec::new());
                }

                byte_buffer.truncate(read);
                let mut bytes = Vec::with_capacity(byte_buffer.capacity());
                std::mem::swap(byte_buffer, &mut bytes);
                Ok(bytes)
            }
            FileStream::Writer { .. } => Err(NodiaError::runtime(
                "read_bytes() expects readable stream, got writable stream",
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
                textcodec::decode_utf8_io(bytes, &format!("cannot read line from stream {id}"))
                    .map(Some)
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

    /// Writes raw bytes to a writable stream without appending a newline.
    pub fn write_bytes(&mut self, stream: StreamId, bytes: &[u8]) -> NodiaResult<()> {
        let id = expect_file_stream(stream, "write_bytes")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Writer { writer } => writer
                .write_all(bytes)
                .map_err(|err| NodiaError::io(format!("cannot write to stream {id}: {err}"))),
            FileStream::Reader { .. } => Err(NodiaError::runtime(
                "write_bytes() expects writable stream, got readable stream",
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
        for conn in self.tcp_streams.values_mut() {
            conn.stream
                .flush()
                .map_err(|err| NodiaError::io(format!("cannot flush tcp stream: {err}")))?;
        }
        Ok(())
    }

    /// Dials a TCP connection to `addr`.
    pub fn dial(&mut self, addr: &str) -> NodiaResult<StreamId> {
        let stream = TcpStream::connect(addr)
            .map_err(|err| NodiaError::io(format!("net.dial '{}': {err}", addr)))?;
        let id = self.next_tcp_id;
        self.next_tcp_id += 1;
        self.tcp_streams.insert(
            id,
            TcpConnection {
                stream,
                read_buffer: Vec::new(),
                eof: false,
            },
        );
        Ok(StreamId::Tcp(id))
    }

    /// Starts listening on `addr`.
    pub fn listen(&mut self, addr: &str) -> NodiaResult<StreamId> {
        let listener = StdTcpListener::bind(addr)
            .map_err(|err| NodiaError::io(format!("net.listen '{}': {err}", addr)))?;
        let id = self.next_tcp_id;
        self.next_tcp_id += 1;
        self.tcp_listeners.insert(id, listener);
        Ok(StreamId::TcpListener(id))
    }

    /// Accepts a connection from a listener.
    pub fn accept(&mut self, listener_id: StreamId) -> NodiaResult<StreamId> {
        let StreamId::TcpListener(listener_id) = listener_id else {
            return Err(NodiaError::runtime("net.accept expects a listener"));
        };
        let listener = self
            .tcp_listeners
            .get(&listener_id)
            .ok_or_else(|| NodiaError::runtime(format!("listener {listener_id} is closed")))?;
        let (stream, _) = listener
            .accept()
            .map_err(|err| NodiaError::io(format!("net.accept: {err}")))?;
        let id = self.next_tcp_id;
        self.next_tcp_id += 1;
        self.tcp_streams.insert(
            id,
            TcpConnection {
                stream,
                read_buffer: Vec::new(),
                eof: false,
            },
        );
        Ok(StreamId::Tcp(id))
    }

    /// Closes a TCP stream or listener.
    pub fn close_tcp(&mut self, stream: StreamId) -> NodiaResult<()> {
        match stream {
            StreamId::Tcp(id) => {
                self.tcp_streams
                    .remove(&id)
                    .ok_or_else(|| NodiaError::runtime(format!("tcp stream {id} is closed")))?;
                Ok(())
            }
            StreamId::TcpListener(id) => {
                self.tcp_listeners
                    .remove(&id)
                    .ok_or_else(|| NodiaError::runtime(format!("listener {id} is closed")))?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn tcp_connection(&mut self, id: usize) -> NodiaResult<&mut TcpConnection> {
        self.tcp_streams
            .get_mut(&id)
            .ok_or_else(|| NodiaError::runtime(format!("tcp stream {id} is closed")))
    }

    pub(super) fn tcp_read_all(&mut self, id: usize) -> NodiaResult<String> {
        let conn = self.tcp_connection(id)?;
        conn.read_buffer.clear();
        conn.stream
            .read_to_end(&mut conn.read_buffer)
            .map_err(|err| NodiaError::io(format!("cannot read tcp {id}: {err}")))?;
        conn.eof = true;
        textcodec::decode_utf8_io(
            std::mem::take(&mut conn.read_buffer),
            &format!("cannot read tcp {id}"),
        )
    }

    pub(super) fn tcp_read_all_bytes(&mut self, id: usize) -> NodiaResult<Vec<u8>> {
        let conn = self.tcp_connection(id)?;
        conn.read_buffer.clear();
        conn.stream
            .read_to_end(&mut conn.read_buffer)
            .map_err(|err| NodiaError::io(format!("cannot read tcp {id}: {err}")))?;
        conn.eof = true;
        Ok(std::mem::take(&mut conn.read_buffer))
    }

    pub(super) fn tcp_read_line(&mut self, id: usize) -> NodiaResult<Option<String>> {
        let conn = self.tcp_connection(id)?;
        conn.read_buffer.clear();
        let mut byte = [0u8; 1];
        loop {
            match conn.stream.read(&mut byte) {
                Ok(0) => {
                    conn.eof = true;
                    if conn.read_buffer.is_empty() {
                        return Ok(None);
                    }
                    let bytes = std::mem::take(&mut conn.read_buffer);
                    return textcodec::decode_utf8_io(
                        bytes,
                        &format!("cannot read line from tcp {id}"),
                    )
                    .map(Some);
                }
                Ok(_) => {
                    conn.read_buffer.push(byte[0]);
                    if byte[0] == b'\n' {
                        let mut bytes = std::mem::take(&mut conn.read_buffer);
                        if bytes.ends_with(b"\n") {
                            bytes.pop();
                            if bytes.ends_with(b"\r") {
                                bytes.pop();
                            }
                        }
                        return textcodec::decode_utf8_io(
                            bytes,
                            &format!("cannot read line from tcp {id}"),
                        )
                        .map(Some);
                    }
                }
                Err(err) => {
                    return Err(NodiaError::io(format!("cannot read from tcp {id}: {err}")));
                }
            }
        }
    }

    pub(super) fn tcp_write(&mut self, id: usize, text: &str) -> NodiaResult<()> {
        let conn = self.tcp_connection(id)?;
        conn.stream
            .write_all(text.as_bytes())
            .map_err(|err| NodiaError::io(format!("cannot write to tcp {id}: {err}")))
    }

    pub(super) fn tcp_eof(&self, id: usize) -> NodiaResult<bool> {
        self.tcp_streams
            .get(&id)
            .map(|conn| conn.eof)
            .ok_or_else(|| NodiaError::runtime(format!("tcp stream {id} is closed")))
    }

    fn stream_mut(&mut self, id: usize) -> NodiaResult<&mut FileStream> {
        self.streams
            .get_mut(&id)
            .ok_or_else(|| NodiaError::runtime(format!("stream {id} is closed")))
    }
}

/// Reads an entire file path into memory.
pub fn read_path(path: &str) -> NodiaResult<String> {
    let bytes = read_path_bytes(path)?;
    textcodec::decode_utf8_io(bytes, &format!("cannot read '{path}'"))
}

/// Reads an entire file path into memory as raw bytes.
pub fn read_path_bytes(path: &str) -> NodiaResult<Vec<u8>> {
    let mut file =
        File::open(path).map_err(|err| NodiaError::io(format!("cannot read '{path}': {err}")))?;
    let mut bytes = Vec::with_capacity(file_size_hint(&file));
    file.read_to_end(&mut bytes)
        .map_err(|err| NodiaError::io(format!("cannot read '{path}': {err}")))?;
    Ok(bytes)
}

/// Writes a full string to a file path, replacing existing content.
pub fn write_path(path: &str, text: &str, allow_write: bool) -> NodiaResult<()> {
    ensure_write_allowed(allow_write)?;
    fs::write(path, text).map_err(|err| NodiaError::io(format!("cannot write '{path}': {err}")))
}

/// Writes raw bytes to a file path, replacing existing content.
pub fn write_path_bytes(path: &str, bytes: &[u8], allow_write: bool) -> NodiaResult<()> {
    ensure_write_allowed(allow_write)?;
    fs::write(path, bytes).map_err(|err| NodiaError::io(format!("cannot write '{path}': {err}")))
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

/// Appends raw bytes to a file path.
pub fn append_path_bytes(path: &str, bytes: &[u8], allow_write: bool) -> NodiaResult<()> {
    ensure_write_allowed(allow_write)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| NodiaError::io(format!("cannot append '{path}': {err}")))?;
    file.write_all(bytes)
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
        StreamId::Tcp(_) | StreamId::TcpListener(_) => Err(NodiaError::runtime(format!(
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
