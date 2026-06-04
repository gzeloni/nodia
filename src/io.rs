use crate::error::{DobraError, DobraResult};
use crate::value::StreamId;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, Write};

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
        chunk_buffer: Vec<u8>,
        line_buffer: String,
    },
    Writer {
        writer: BufWriter<File>,
    },
}

impl IoRegistry {
    pub fn new() -> Self {
        Self {
            next_file_id: 1,
            streams: HashMap::new(),
        }
    }

    pub fn open(&mut self, path: &str, mode: &str, allow_write: bool) -> DobraResult<StreamId> {
        let stream = match mode {
            "read" => FileStream::Reader {
                reader: BufReader::new(File::open(path).map_err(|err| {
                    DobraError::io(format!("cannot open '{path}' for read: {err}"))
                })?),
                eof: false,
                chunk_buffer: Vec::new(),
                line_buffer: String::new(),
            },
            "write" => {
                ensure_write_allowed(allow_write)?;
                FileStream::Writer {
                    writer: BufWriter::new(File::create(path).map_err(|err| {
                        DobraError::io(format!("cannot open '{path}' for write: {err}"))
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
                                DobraError::io(format!("cannot open '{path}' for append: {err}"))
                            })?,
                    ),
                }
            }
            _ => {
                return Err(DobraError::runtime(format!(
                    "open() mode must be 'read', 'write' or 'append', got '{mode}'"
                )))
            }
        };

        let id = self.next_file_id;
        self.next_file_id += 1;
        self.streams.insert(id, stream);
        Ok(StreamId::File(id))
    }

    pub fn close(&mut self, stream: StreamId) -> DobraResult<()> {
        let StreamId::File(id) = stream else {
            return Ok(());
        };
        let Some(mut stream) = self.streams.remove(&id) else {
            return Err(DobraError::runtime(format!("stream {id} is closed")));
        };
        if let FileStream::Writer { writer } = &mut stream {
            writer.flush().map_err(|err| {
                DobraError::io(format!("cannot flush stream {id} before close: {err}"))
            })?;
        }
        Ok(())
    }

    pub fn flush(&mut self, stream: StreamId) -> DobraResult<()> {
        let id = expect_file_stream(stream, "flush")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Writer { writer } => writer
                .flush()
                .map_err(|err| DobraError::io(format!("cannot flush stream {id}: {err}"))),
            FileStream::Reader { .. } => Err(DobraError::runtime(
                "flush() expects writable stream, got readable stream",
            )),
        }
    }

    pub fn eof(&mut self, stream: StreamId) -> DobraResult<bool> {
        let id = expect_file_stream(stream, "eof")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader { eof, .. } => Ok(*eof),
            FileStream::Writer { .. } => Err(DobraError::runtime(
                "eof() expects readable stream, got writable stream",
            )),
        }
    }

    pub fn read_all(&mut self, stream: StreamId) -> DobraResult<String> {
        let id = expect_file_stream(stream, "read")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader { reader, eof, .. } => {
                let mut out = Vec::with_capacity(remaining_file_hint(reader));
                reader.read_to_end(&mut out).map_err(|err| {
                    DobraError::io(format!("cannot read from stream {id}: {err}"))
                })?;
                *eof = true;
                into_utf8_string(out, &format!("cannot read from stream {id}"))
            }
            FileStream::Writer { .. } => Err(DobraError::runtime(
                "read() expects readable stream, got writable stream",
            )),
        }
    }

    pub fn read_chunk(&mut self, stream: StreamId, size: usize) -> DobraResult<String> {
        let id = expect_file_stream(stream, "read")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader {
                reader,
                eof,
                chunk_buffer,
                ..
            } => {
                chunk_buffer.clear();
                chunk_buffer.resize(size, 0);
                let read = reader.read(chunk_buffer).map_err(|err| {
                    DobraError::io(format!("cannot read from stream {id}: {err}"))
                })?;
                if read == 0 {
                    *eof = true;
                    chunk_buffer.clear();
                    return Ok(String::new());
                }
                chunk_buffer.truncate(read);
                let mut bytes = Vec::with_capacity(chunk_buffer.capacity());
                std::mem::swap(chunk_buffer, &mut bytes);
                match String::from_utf8(bytes) {
                    Ok(text) => Ok(text),
                    Err(err) => Ok(String::from_utf8_lossy(&err.into_bytes()).into_owned()),
                }
            }
            FileStream::Writer { .. } => Err(DobraError::runtime(
                "read() expects readable stream, got writable stream",
            )),
        }
    }

    pub fn read_line(&mut self, stream: StreamId) -> DobraResult<Option<String>> {
        let id = expect_file_stream(stream, "readln")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Reader {
                reader,
                eof,
                line_buffer,
                ..
            } => {
                line_buffer.clear();
                let read = reader.read_line(line_buffer).map_err(|err| {
                    DobraError::io(format!("cannot read line from stream {id}: {err}"))
                })?;
                if read == 0 {
                    *eof = true;
                    return Ok(None);
                }
                if line_buffer.ends_with('\n') {
                    line_buffer.pop();
                    if line_buffer.ends_with('\r') {
                        line_buffer.pop();
                    }
                }
                let mut line = String::with_capacity(line_buffer.capacity());
                std::mem::swap(line_buffer, &mut line);
                Ok(Some(line))
            }
            FileStream::Writer { .. } => Err(DobraError::runtime(
                "readln() expects readable stream, got writable stream",
            )),
        }
    }

    pub fn write(&mut self, stream: StreamId, text: &str) -> DobraResult<()> {
        let id = expect_file_stream(stream, "write")?;
        let stream = self.stream_mut(id)?;
        match stream {
            FileStream::Writer { writer } => writer
                .write_all(text.as_bytes())
                .map_err(|err| DobraError::io(format!("cannot write to stream {id}: {err}"))),
            FileStream::Reader { .. } => Err(DobraError::runtime(
                "write() expects writable stream, got readable stream",
            )),
        }
    }

    pub fn flush_all(&mut self) -> DobraResult<()> {
        for (id, stream) in &mut self.streams {
            if let FileStream::Writer { writer } = stream {
                writer
                    .flush()
                    .map_err(|err| DobraError::io(format!("cannot flush stream {id}: {err}")))?;
            }
        }
        Ok(())
    }

    fn stream_mut(&mut self, id: usize) -> DobraResult<&mut FileStream> {
        self.streams
            .get_mut(&id)
            .ok_or_else(|| DobraError::runtime(format!("stream {id} is closed")))
    }
}

pub fn read_path(path: &str) -> DobraResult<String> {
    let mut file =
        File::open(path).map_err(|err| DobraError::io(format!("cannot read '{path}': {err}")))?;
    let mut bytes = Vec::with_capacity(file_size_hint(&file));
    file.read_to_end(&mut bytes)
        .map_err(|err| DobraError::io(format!("cannot read '{path}': {err}")))?;
    into_utf8_string(bytes, &format!("cannot read '{path}'"))
}

pub fn write_path(path: &str, text: &str, allow_write: bool) -> DobraResult<()> {
    ensure_write_allowed(allow_write)?;
    fs::write(path, text).map_err(|err| DobraError::io(format!("cannot write '{path}': {err}")))
}

pub fn append_path(path: &str, text: &str, allow_write: bool) -> DobraResult<()> {
    ensure_write_allowed(allow_write)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| DobraError::io(format!("cannot append '{path}': {err}")))?;
    file.write_all(text.as_bytes())
        .map_err(|err| DobraError::io(format!("cannot append '{path}': {err}")))
}

pub fn ensure_write_allowed(allow_write: bool) -> DobraResult<()> {
    if allow_write {
        Ok(())
    } else {
        Err(DobraError::io("file write requires --allow-write").with_code("E3001"))
    }
}

fn expect_file_stream(stream: StreamId, name: &str) -> DobraResult<usize> {
    match stream {
        StreamId::File(id) => Ok(id),
        StreamId::Stdin | StreamId::Stdout | StreamId::Stderr => Err(DobraError::runtime(format!(
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

fn into_utf8_string(bytes: Vec<u8>, context: &str) -> DobraResult<String> {
    String::from_utf8(bytes).map_err(|err| DobraError::io(format!("{context}: {err}")))
}
