// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Runtime bindings for I/O, process, and environment built-ins.

use super::*;
use crate::textcodec;
use crate::value::LazyValue;

impl Runtime {
    pub(super) fn call_io_builtin(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> NodiaResult<Option<Value>> {
        let result = match name {
            "open" => {
                self.expect_arity(args, 2, "open")?;
                let path = self.expect_string(&args[0], "open", "first")?;
                let mode = self.expect_string(&args[1], "open", "second")?;
                Self::io_pipeline_value(
                    "io.open",
                    self.io
                        .borrow_mut()
                        .open(&path, &mode, self.options.allow_write)
                        .map(Value::Stream),
                )?
            }
            "close" => {
                self.expect_arity(args, 1, "close")?;
                let stream = self.expect_stream(&args[0], "close", "first")?;
                Self::io_pipeline_value("io.close", self.close_stream(stream).map(|_| Value::Null))?
            }
            "flush" => {
                self.expect_arity(args, 1, "flush")?;
                let stream = self.expect_stream(&args[0], "flush", "first")?;
                Self::io_pipeline_value("io.flush", self.flush_stream(stream).map(|_| Value::Null))?
            }
            "eof" => {
                self.expect_arity(args, 1, "eof")?;
                let stream = self.expect_stream(&args[0], "eof", "first")?;
                Self::io_pipeline_value("io.eof", self.eof_stream(stream).map(Value::Bool))?
            }
            "read" => Self::io_pipeline_value("io.read", self.read_builtin(args))?,
            "readln" => {
                self.expect_arity(args, 1, "readln")?;
                let stream = self.expect_stream(&args[0], "readln", "first")?;
                Self::io_pipeline_value(
                    "io.readln",
                    self.read_line_stream(stream).map(|line| match line {
                        Some(line) => Value::String(line),
                        None => Value::Null,
                    }),
                )?
            }
            "io.lines" => {
                self.expect_arity(args, 1, "io.lines")?;
                let stream = self.expect_stream(&args[0], "io.lines", "first")?;
                Value::Lazy(LazyValue::lines(stream))
            }
            "io.chunks" => self.chunks_builtin(args)?,
            "write" => Self::io_pipeline_value("io.write", self.write_builtin(args, false))?,
            "writeln" => Self::io_pipeline_value("io.writeln", self.write_builtin(args, true))?,
            "append" => {
                self.expect_arity(args, 2, "append")?;
                let path = self.expect_string(&args[0], "append", "first")?;
                let outcome = match &args[1] {
                    Value::Bytes(bytes) => {
                        fsio::append_path_bytes(&path, bytes, self.options.allow_write)
                    }
                    value => fsio::append_path(&path, &value.to_string(), self.options.allow_write),
                };
                Self::io_pipeline_value("io.append", outcome.map(|_| Value::Null))?
            }
            "net.dial" => {
                self.expect_arity(args, 1, "net.dial")?;
                let addr = self.expect_string(&args[0], "net.dial", "first")?;
                Self::io_pipeline_value(
                    "net.dial",
                    self.io.borrow_mut().dial(&addr).map(Value::Stream),
                )?
            }
            "net.listen" => {
                self.expect_arity(args, 1, "net.listen")?;
                let addr = self.expect_string(&args[0], "net.listen", "first")?;
                Self::io_pipeline_value(
                    "net.listen",
                    self.io.borrow_mut().listen(&addr).map(Value::Stream),
                )?
            }
            "net.accept" => {
                self.expect_arity(args, 1, "net.accept")?;
                let listener = self.expect_stream(&args[0], "net.accept", "first")?;
                Self::io_pipeline_value(
                    "net.accept",
                    self.io.borrow_mut().accept(listener).map(Value::Stream),
                )?
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn io_pipeline_value(context: &str, outcome: NodiaResult<Value>) -> NodiaResult<Value> {
        match outcome {
            Ok(value) => Ok(value),
            Err(error) if error.code.starts_with("E2") => Err(error),
            Err(error) => Err(error.with_context(context)),
        }
    }

    pub(super) fn call_runtime_builtin(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> NodiaResult<Option<Value>> {
        let result = match name {
            "env" => self.env_builtin(args)?,
            "exit" => return Err(NodiaError::exit(self.exit_builtin(args)?)),
            "exec" => self.exec_builtin(args)?,
            "map" => self.map_builtin(args)?,
            "filter" => self.filter_builtin(args)?,
            "reduce" => self.reduce_builtin(args)?,
            "collect" => self.collect_builtin(args)?,
            "group_by" => self.group_by_builtin(args)?,
            "sort_by" => self.sort_by_builtin(args)?,
            "math.random" => self.math_random_builtin(args)?,
            "math.random_int" => self.math_random_int_builtin(args)?,
            "base64.encode" => base64_encode_builtin(args)?,
            "base64.decode" => base64_decode_builtin(args)?,
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    pub(super) fn env_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        if args.len() != 1 && args.len() != 2 {
            return Err(NodiaError::runtime(format!(
                "env() expects 1 or 2 argument(s), got {}",
                args.len()
            )));
        }
        if !self.options.allow_env {
            return Err(
                NodiaError::io("environment access requires --allow-env").with_code("E3002")
            );
        }

        let name = self.expect_string(&args[0], "env", "first")?;
        match std::env::var(&name) {
            Ok(value) => Ok(Value::String(value)),
            Err(std::env::VarError::NotPresent) => {
                if let Some(default) = args.get(1) {
                    Ok(default.clone())
                } else {
                    Ok(Value::Null)
                }
            }
            Err(std::env::VarError::NotUnicode(_)) => Err(NodiaError::io(format!(
                "environment variable '{name}' is not valid unicode"
            ))),
        }
    }

    pub(super) fn exit_builtin(&self, args: &[Value]) -> NodiaResult<i32> {
        if args.len() > 1 {
            return Err(NodiaError::runtime(format!(
                "exit() expects 0 or 1 argument(s), got {}",
                args.len()
            )));
        }
        let status = if let Some(value) = args.first() {
            self.expect_exit_code(value)?
        } else {
            0
        };
        Ok(status)
    }

    pub(super) fn exec_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        if args.len() != 1 && args.len() != 2 {
            return Err(NodiaError::runtime(format!(
                "exec() expects 1 or 2 argument(s), got {}",
                args.len()
            )));
        }
        if !self.options.allow_process {
            return Err(
                NodiaError::io("process execution requires --allow-process").with_code("E3003")
            );
        }

        let command = self.expect_string(&args[0], "exec", "first")?;
        let mut child = Command::new(&command);
        if let Some(values) = args.get(1) {
            let values = self.expect_list_value(values, "exec", "second")?;
            for value in values {
                child.arg(value.to_string());
            }
        }

        let mut result = BTreeMap::new();
        match child.output() {
            Ok(output) => {
                result.insert(
                    "stdout".to_string(),
                    textcodec::bytes_to_value(output.stdout),
                );
                result.insert(
                    "stderr".to_string(),
                    textcodec::bytes_to_value(output.stderr),
                );
                result.insert(
                    "status".to_string(),
                    Value::Int(output.status.code().unwrap_or(-1) as i64),
                );
            }
            Err(err) => {
                result.insert("stdout".to_string(), Value::Bytes(Vec::new()));
                result.insert("stderr".to_string(), Value::Bytes(Vec::new()));
                result.insert("status".to_string(), Value::Int(-1));
                result.insert("error".to_string(), Value::String(err.to_string()));
            }
        }
        Ok(Value::Map(result))
    }

    pub(super) fn map_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "map")?;
        let function = self.expect_callable(&args[0], "map", "first")?;
        if let Value::Lazy(source) = &args[1] {
            return Ok(Value::Lazy(LazyValue::map(source.clone(), function)));
        }
        let values = self.iterable_single_values(args[1].clone())?;
        let mut mapped = Vec::with_capacity(values.len());
        for value in values {
            mapped.push(self.invoke_callable1(function.clone(), value)?);
        }
        Ok(Value::List(mapped))
    }

    pub(super) fn filter_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "filter")?;
        let function = self.expect_callable(&args[0], "filter", "first")?;
        if let Value::Lazy(source) = &args[1] {
            return Ok(Value::Lazy(LazyValue::filter(source.clone(), function)));
        }
        let values = self.iterable_single_values(args[1].clone())?;
        let mut filtered = Vec::new();
        for value in values {
            if self
                .invoke_callable1(function.clone(), value.clone())?
                .truthy()
            {
                filtered.push(value);
            }
        }
        Ok(Value::List(filtered))
    }

    pub(super) fn reduce_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 3, "reduce")?;
        let function = self.expect_callable(&args[0], "reduce", "first")?;
        let mut accumulator = args[1].clone();
        match args[2].clone() {
            Value::Lazy(lazy) => {
                while let Some(value) = self.next_lazy_value(&lazy)? {
                    accumulator = self.invoke_callable2(function.clone(), accumulator, value)?;
                }
            }
            other => {
                for value in self.iterable_single_values(other)? {
                    accumulator = self.invoke_callable2(function.clone(), accumulator, value)?;
                }
            }
        }
        Ok(accumulator)
    }

    pub(super) fn collect_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 1, "collect")?;
        Ok(Value::List(self.collect_iterable_values(args[0].clone())?))
    }

    pub(super) fn group_by_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "group_by")?;
        let function = self.expect_callable(&args[0], "group_by", "first")?;
        let values = self.collect_iterable_values(args[1].clone())?;
        let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for value in values {
            let key = self
                .invoke_callable1(function.clone(), value.clone())?
                .to_string();
            groups.entry(key).or_default().push(value.clone());
        }

        Ok(Value::Map(
            groups
                .into_iter()
                .map(|(key, values)| (key, Value::List(values)))
                .collect(),
        ))
    }

    pub(super) fn sort_by_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "sort_by")?;
        let function = self.expect_callable(&args[0], "sort_by", "first")?;
        let values = self.collect_iterable_values(args[1].clone())?;
        let mut decorated = Vec::with_capacity(values.len());
        for (index, value) in values.iter().cloned().enumerate() {
            let key = self.invoke_callable1(function.clone(), value.clone())?;
            decorated.push((index, key, value));
        }
        decorated.sort_by(|(left_index, left_key, _), (right_index, right_key, _)| {
            stdlib::compare_values(left_key, right_key).then_with(|| left_index.cmp(right_index))
        });
        Ok(Value::List(
            decorated.into_iter().map(|(_, _, value)| value).collect(),
        ))
    }

    fn next_random_u64(&mut self) -> u64 {
        let mut x = self.prng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.prng_state = x;
        x
    }

    fn random_float(&mut self) -> f64 {
        (self.next_random_u64() as f64) / (u64::MAX as f64)
    }

    fn random_int(&mut self, min: i64, max: i64) -> i64 {
        if min > max {
            return min;
        }
        let range = (max - min + 1) as u64;
        if range == 0 {
            return min;
        }
        (self.next_random_u64() % range) as i64 + min
    }

    pub(super) fn math_random_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 0, "math.random")?;
        Ok(Value::Float(self.random_float()))
    }

    pub(super) fn math_random_int_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "math.random_int")?;
        let min = match &args[0] {
            Value::Int(v) => *v,
            other => {
                return Err(NodiaError::runtime(format!(
                    "math.random_int() expects int as first argument, got {}",
                    other.type_name()
                )));
            }
        };
        let max = match &args[1] {
            Value::Int(v) => *v,
            other => {
                return Err(NodiaError::runtime(format!(
                    "math.random_int() expects int as second argument, got {}",
                    other.type_name()
                )));
            }
        };
        Ok(Value::Int(self.random_int(min, max)))
    }

    fn chunks_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        match args {
            [stream, size] => {
                let stream = self.expect_stream(stream, "io.chunks", "first")?;
                let size = self.expect_positive_size(size, "io.chunks", "second")?;
                Ok(Value::Lazy(LazyValue::text_chunks(stream, size)))
            }
            [stream, mode, size] => {
                let stream = self.expect_stream(stream, "io.chunks", "first")?;
                let size = self.expect_positive_size(size, "io.chunks", "third")?;
                let kind = expect_read_kind(mode, "io.chunks", "second")?;
                match kind {
                    ReadKind::Text => Ok(Value::Lazy(LazyValue::text_chunks(stream, size))),
                    ReadKind::Bytes => Ok(Value::Lazy(LazyValue::byte_chunks(stream, size))),
                }
            }
            _ => Err(NodiaError::runtime(format!(
                "io.chunks() expects 2 or 3 argument(s), got {}",
                args.len()
            ))),
        }
    }

    pub(super) fn read_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        match args {
            [source] => self.read_value(source, ReadKind::Text),
            [source, Value::Int(_)] => {
                let stream = self.expect_stream(source, "read", "first")?;
                let size = self.expect_non_negative_size(&args[1], "read", "second")?;
                self.read_chunk_value(stream, size, ReadKind::Text)
            }
            [source, mode] => self.read_value(source, expect_read_kind(mode, "read", "second")?),
            [source, mode, size] => {
                let stream = self.expect_stream(source, "read", "first")?;
                let size = self.expect_non_negative_size(size, "read", "third")?;
                self.read_chunk_value(stream, size, expect_read_kind(mode, "read", "second")?)
            }
            _ => Err(NodiaError::runtime(format!(
                "read() expects 1, 2, or 3 argument(s), got {}",
                args.len()
            ))),
        }
    }

    pub(super) fn write_builtin(&mut self, args: &[Value], line: bool) -> NodiaResult<Value> {
        self.expect_arity(args, 2, if line { "writeln" } else { "write" })?;
        if line {
            let mut text = args[1].to_string();
            text.push('\n');
            match &args[0] {
                Value::String(_) => {
                    return Err(NodiaError::runtime(
                        "writeln() expects stream as first argument",
                    ));
                }
                Value::Stream(stream) => self.write_stream(*stream, &text)?,
                other => {
                    return Err(NodiaError::runtime(format!(
                        "writeln() expects path or stream, got {}",
                        other.type_name()
                    )));
                }
            }
            return Ok(Value::Null);
        }

        match (&args[0], &args[1]) {
            (Value::String(path), Value::Bytes(bytes)) => {
                fsio::write_path_bytes(path, bytes, self.options.allow_write)?;
            }
            (Value::Stream(stream), Value::Bytes(bytes)) => {
                self.write_bytes_stream(*stream, bytes)?;
            }
            (Value::String(path), value) => {
                fsio::write_path(path, &value.to_string(), self.options.allow_write)?;
            }
            (Value::Stream(stream), value) => self.write_stream(*stream, &value.to_string())?,
            (other, _) => {
                return Err(NodiaError::runtime(format!(
                    "write() expects path or stream, got {}",
                    other.type_name()
                )));
            }
        }
        Ok(Value::Null)
    }

    fn read_value(&mut self, source: &Value, kind: ReadKind) -> NodiaResult<Value> {
        match (source, kind) {
            (Value::String(path), ReadKind::Text) => fsio::read_path(path).map(Value::String),
            (Value::String(path), ReadKind::Bytes) => {
                fsio::read_path_bytes(path).map(textcodec::bytes_to_value)
            }
            (Value::Stream(stream), ReadKind::Text) => self.read_stream(*stream).map(Value::String),
            (Value::Stream(stream), ReadKind::Bytes) => self
                .read_bytes_stream(*stream)
                .map(textcodec::bytes_to_value),
            (other, _) => Err(NodiaError::runtime(format!(
                "read() expects path or stream, got {}",
                other.type_name()
            ))),
        }
    }

    fn read_chunk_value(
        &mut self,
        stream: StreamId,
        size: usize,
        kind: ReadKind,
    ) -> NodiaResult<Value> {
        match kind {
            ReadKind::Text => self.read_chunk_stream(stream, size).map(Value::String),
            ReadKind::Bytes => self
                .read_chunk_bytes_stream(stream, size)
                .map(textcodec::bytes_to_value),
        }
    }

    pub(super) fn read_stream(&mut self, stream: StreamId) -> NodiaResult<String> {
        match stream {
            StreamId::Stdin => {
                self.flush_output_channel()?;
                let mut input = String::new();
                stdio::stdin()
                    .lock()
                    .read_to_string(&mut input)
                    .map_err(|err| NodiaError::io(format!("cannot read stdin: {err}")))?;
                Ok(input)
            }
            StreamId::Stdout => Err(NodiaError::runtime("cannot read from stdout")),
            StreamId::Stderr => Err(NodiaError::runtime("cannot read from stderr")),
            StreamId::File(_) => self.io.borrow_mut().read_all(stream),
            StreamId::Tcp(id) => self.io.borrow_mut().tcp_read_all(id),
            StreamId::TcpListener(_) => Err(NodiaError::runtime("cannot read from a listener")),
        }
    }

    pub(super) fn read_chunk_stream(
        &mut self,
        stream: StreamId,
        size: usize,
    ) -> NodiaResult<String> {
        match stream {
            StreamId::Stdin => {
                self.flush_output_channel()?;
                let mut buffer = vec![0; size];
                let read = stdio::stdin()
                    .lock()
                    .read(&mut buffer)
                    .map_err(|err| NodiaError::io(format!("cannot read stdin: {err}")))?;
                buffer.truncate(read);
                textcodec::decode_utf8_io(buffer, "cannot read stdin")
            }
            StreamId::Stdout => Err(NodiaError::runtime("cannot read from stdout")),
            StreamId::Stderr => Err(NodiaError::runtime("cannot read from stderr")),
            StreamId::File(_) => self.io.borrow_mut().read_chunk(stream, size),
            StreamId::Tcp(_) => self.read_stream(stream), // TCP reads all, chunked not supported
            StreamId::TcpListener(_) => Err(NodiaError::runtime("cannot read from a listener")),
        }
    }

    pub(super) fn read_bytes_stream(&mut self, stream: StreamId) -> NodiaResult<Vec<u8>> {
        match stream {
            StreamId::Stdin => {
                self.flush_output_channel()?;
                let mut buffer = Vec::new();
                stdio::stdin()
                    .lock()
                    .read_to_end(&mut buffer)
                    .map_err(|err| NodiaError::io(format!("cannot read stdin: {err}")))?;
                Ok(buffer)
            }
            StreamId::Stdout => Err(NodiaError::runtime("cannot read from stdout")),
            StreamId::Stderr => Err(NodiaError::runtime("cannot read from stderr")),
            StreamId::File(_) => self.io.borrow_mut().read_all_bytes(stream),
            StreamId::Tcp(id) => self.io.borrow_mut().tcp_read_all_bytes(id),
            StreamId::TcpListener(_) => Err(NodiaError::runtime("cannot read from a listener")),
        }
    }

    pub(super) fn read_chunk_bytes_stream(
        &mut self,
        stream: StreamId,
        size: usize,
    ) -> NodiaResult<Vec<u8>> {
        match stream {
            StreamId::Stdin => {
                self.flush_output_channel()?;
                if size == 0 {
                    return Ok(Vec::new());
                }
                let mut buffer = vec![0; size];
                let read = stdio::stdin()
                    .lock()
                    .read(&mut buffer)
                    .map_err(|err| NodiaError::io(format!("cannot read stdin: {err}")))?;
                buffer.truncate(read);
                Ok(buffer)
            }
            StreamId::Stdout => Err(NodiaError::runtime("cannot read from stdout")),
            StreamId::Stderr => Err(NodiaError::runtime("cannot read from stderr")),
            StreamId::File(_) => self.io.borrow_mut().read_chunk_bytes(stream, size),
            StreamId::Tcp(_) => self.read_bytes_stream(stream), // TCP reads all
            StreamId::TcpListener(_) => Err(NodiaError::runtime("cannot read from a listener")),
        }
    }

    pub(super) fn read_line_stream(&mut self, stream: StreamId) -> NodiaResult<Option<String>> {
        match stream {
            StreamId::Stdin => {
                self.flush_output_channel()?;
                let mut line = String::new();
                let read = stdio::stdin()
                    .lock()
                    .read_line(&mut line)
                    .map_err(|err| NodiaError::io(format!("cannot read stdin: {err}")))?;
                if read == 0 {
                    return Ok(None);
                }
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                Ok(Some(line))
            }
            StreamId::Stdout => Err(NodiaError::runtime("cannot read from stdout")),
            StreamId::Stderr => Err(NodiaError::runtime("cannot read from stderr")),
            StreamId::File(_) => self.io.borrow_mut().read_line(stream),
            StreamId::Tcp(id) => self.io.borrow_mut().tcp_read_line(id),
            StreamId::TcpListener(_) => Err(NodiaError::runtime("cannot read from a listener")),
        }
    }

    pub(super) fn write_stream(&mut self, stream: StreamId, text: &str) -> NodiaResult<()> {
        match stream {
            StreamId::Stdin => Err(NodiaError::runtime("cannot write to stdin")),
            StreamId::Stdout => self.write_output_channel(text),
            StreamId::Stderr => stdio::stderr()
                .write_all(text.as_bytes())
                .map_err(|err| NodiaError::io(format!("cannot write stderr: {err}"))),
            StreamId::File(_) => self.io.borrow_mut().write(stream, text),
            StreamId::Tcp(id) => self.io.borrow_mut().tcp_write(id, text),
            StreamId::TcpListener(_) => Err(NodiaError::runtime("cannot write to a listener")),
        }
    }

    pub(super) fn write_bytes_stream(&mut self, stream: StreamId, bytes: &[u8]) -> NodiaResult<()> {
        match stream {
            StreamId::Stdin => Err(NodiaError::runtime("cannot write to stdin")),
            StreamId::Stdout => Err(NodiaError::runtime(
                "cannot write raw bytes to stdout; stdout remains a text channel",
            )),
            StreamId::Stderr => stdio::stderr()
                .write_all(bytes)
                .map_err(|err| NodiaError::io(format!("cannot write stderr: {err}"))),
            StreamId::File(_) => self.io.borrow_mut().write_bytes(stream, bytes),
            StreamId::Tcp(id) => self
                .io
                .borrow_mut()
                .tcp_write(id, &String::from_utf8_lossy(bytes)),
            StreamId::TcpListener(_) => Err(NodiaError::runtime("cannot write to a listener")),
        }
    }

    pub(super) fn flush_stream(&mut self, stream: StreamId) -> NodiaResult<()> {
        match stream {
            StreamId::Stdin => Ok(()),
            StreamId::Stdout => self.flush_output_channel(),
            StreamId::Stderr => stdio::stderr()
                .flush()
                .map_err(|err| NodiaError::io(format!("cannot flush stderr: {err}"))),
            StreamId::File(_) => self.io.borrow_mut().flush(stream),
            StreamId::Tcp(_) | StreamId::TcpListener(_) => Ok(()), // TCP is auto-flushed
        }
    }

    pub(super) fn close_stream(&mut self, stream: StreamId) -> NodiaResult<()> {
        match stream {
            StreamId::Stdin | StreamId::Stdout => Ok(()),
            StreamId::Stderr => self.flush_stream(stream),
            StreamId::File(_) => self.io.borrow_mut().close(stream),
            StreamId::Tcp(_) | StreamId::TcpListener(_) => self.io.borrow_mut().close_tcp(stream),
        }
    }

    pub(super) fn eof_stream(&mut self, stream: StreamId) -> NodiaResult<bool> {
        match stream {
            StreamId::Stdin => Ok(false),
            StreamId::Stdout | StreamId::Stderr => {
                Err(NodiaError::runtime("eof() expects readable stream"))
            }
            StreamId::File(_) => self.io.borrow_mut().eof(stream),
            StreamId::Tcp(id) => self.io.borrow_mut().tcp_eof(id),
            StreamId::TcpListener(_) => Err(NodiaError::runtime("eof() expects readable stream")),
        }
    }
}

#[derive(Clone, Copy)]
enum ReadKind {
    Text,
    Bytes,
}

fn expect_read_kind(value: &Value, name: &str, position: &str) -> NodiaResult<ReadKind> {
    match value {
        Value::String(mode) => match mode.as_str() {
            "text" => Ok(ReadKind::Text),
            "bytes" => Ok(ReadKind::Bytes),
            other => Err(NodiaError::runtime(format!(
                "{name}() expects text or bytes as {position} argument, got '{other}'"
            ))),
        },
        other => Err(NodiaError::runtime(format!(
            "{name}() expects read mode as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode_builtin(args: &[Value]) -> NodiaResult<Value> {
    if args.len() != 1 {
        return Err(NodiaError::runtime(format!(
            "base64.encode() expects 1 argument(s), got {}",
            args.len()
        )));
    }
    let input: Vec<u8> = match &args[0] {
        Value::Bytes(bytes) => bytes.clone(),
        Value::String(s) => s.as_bytes().to_vec(),
        other => {
            return Err(NodiaError::runtime(format!(
                "base64.encode() expects string or bytes, got {}",
                other.type_name()
            )));
        }
    };
    Ok(Value::String(base64_encode(&input)))
}

fn base64_decode_builtin(args: &[Value]) -> NodiaResult<Value> {
    if args.len() != 1 {
        return Err(NodiaError::runtime(format!(
            "base64.decode() expects 1 argument(s), got {}",
            args.len()
        )));
    }
    let input = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(NodiaError::runtime(format!(
                "base64.decode() expects string, got {}",
                other.type_name()
            )));
        }
    };
    base64_decode(&input)
        .map(Value::Bytes)
        .ok_or_else(|| NodiaError::runtime("base64.decode() invalid base64 input"))
}

fn base64_encode(input: &[u8]) -> String {
    let mut output = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let packed = (b0 << 16) | (b1 << 8) | b2;
        output.push(BASE64_ALPHABET[((packed >> 18) & 0x3F) as usize] as char);
        output.push(BASE64_ALPHABET[((packed >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            output.push(BASE64_ALPHABET[((packed >> 6) & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(BASE64_ALPHABET[(packed & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let input = input.trim_end_matches('=');
    let mut output = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits_collected: u32 = 0;
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | value as u32;
        bits_collected += 6;
        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }
    Some(output)
}
