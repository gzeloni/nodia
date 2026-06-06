// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Runtime bindings for I/O, process, and environment built-ins.

use super::*;
use crate::textcodec;

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
                Value::Stream(
                    self.io
                        .borrow_mut()
                        .open(&path, &mode, self.options.allow_write)?,
                )
            }
            "close" => {
                self.expect_arity(args, 1, "close")?;
                self.close_stream(self.expect_stream(&args[0], "close", "first")?)?;
                Value::Null
            }
            "flush" => {
                self.expect_arity(args, 1, "flush")?;
                self.flush_stream(self.expect_stream(&args[0], "flush", "first")?)?;
                Value::Null
            }
            "eof" => {
                self.expect_arity(args, 1, "eof")?;
                Value::Bool(self.eof_stream(self.expect_stream(&args[0], "eof", "first")?)?)
            }
            "read" => self.read_builtin(args)?,
            "read_bytes" => self.read_bytes_builtin(args)?,
            "readln" => {
                self.expect_arity(args, 1, "readln")?;
                match self.read_line_stream(self.expect_stream(&args[0], "readln", "first")?)? {
                    Some(line) => Value::String(line),
                    None => Value::Null,
                }
            }
            "write" => self.write_builtin(args, false)?,
            "write_bytes" => self.write_bytes_builtin(args)?,
            "writeln" => self.write_builtin(args, true)?,
            "append" => {
                self.expect_arity(args, 2, "append")?;
                let path = self.expect_string(&args[0], "append", "first")?;
                fsio::append_path(&path, &args[1].to_string(), self.options.allow_write)?;
                Value::Null
            }
            "append_bytes" => {
                self.expect_arity(args, 2, "append_bytes")?;
                let path = self.expect_string(&args[0], "append_bytes", "first")?;
                let bytes = textcodec::expect_bytes(&args[1], "append_bytes", "second")?;
                fsio::append_path_bytes(&path, &bytes, self.options.allow_write)?;
                Value::Null
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
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
            "group_by" => self.group_by_builtin(args)?,
            "sort_by" => self.sort_by_builtin(args)?,
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
                result.insert("stdout".to_string(), Value::List(Vec::new()));
                result.insert("stderr".to_string(), Value::List(Vec::new()));
                result.insert("status".to_string(), Value::Int(-1));
                result.insert("error".to_string(), Value::String(err.to_string()));
            }
        }
        Ok(Value::Map(result))
    }

    pub(super) fn map_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "map")?;
        let function = self.expect_callable(&args[0], "map", "first")?;
        let values = self.expect_list_value(&args[1], "map", "second")?;
        let mut mapped = Vec::with_capacity(values.len());
        for value in values {
            mapped.push(self.invoke_callable1(function.clone(), value.clone())?);
        }
        Ok(Value::List(mapped))
    }

    pub(super) fn filter_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "filter")?;
        let function = self.expect_callable(&args[0], "filter", "first")?;
        let values = self.expect_list_value(&args[1], "filter", "second")?;
        let mut filtered = Vec::new();
        for value in values {
            if self
                .invoke_callable1(function.clone(), value.clone())?
                .truthy()
            {
                filtered.push(value.clone());
            }
        }
        Ok(Value::List(filtered))
    }

    pub(super) fn reduce_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 3, "reduce")?;
        let function = self.expect_callable(&args[0], "reduce", "first")?;
        let values = self.expect_list_value(&args[2], "reduce", "third")?;
        let mut accumulator = args[1].clone();
        for value in values {
            accumulator = self.invoke_callable2(function.clone(), accumulator, value.clone())?;
        }
        Ok(accumulator)
    }

    pub(super) fn group_by_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "group_by")?;
        let function = self.expect_callable(&args[0], "group_by", "first")?;
        let values = self.expect_list_value(&args[1], "group_by", "second")?;
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
        let values = self.expect_list_value(&args[1], "sort_by", "second")?;
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

    pub(super) fn read_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        if args.len() == 1 {
            return match &args[0] {
                Value::String(path) => fsio::read_path(path).map(Value::String),
                Value::Stream(stream) => self.read_stream(*stream).map(Value::String),
                other => Err(NodiaError::runtime(format!(
                    "read() expects path or stream, got {}",
                    other.type_name()
                ))),
            };
        }
        if args.len() == 2 {
            let stream = self.expect_stream(&args[0], "read", "first")?;
            let size = self.expect_non_negative_size(&args[1], "read", "second")?;
            return self.read_chunk_stream(stream, size).map(Value::String);
        }
        Err(NodiaError::runtime(format!(
            "read() expects 1 or 2 argument(s), got {}",
            args.len()
        )))
    }

    pub(super) fn write_builtin(&mut self, args: &[Value], line: bool) -> NodiaResult<Value> {
        self.expect_arity(args, 2, if line { "writeln" } else { "write" })?;
        let mut text = args[1].to_string();
        if line {
            text.push('\n');
        }
        match &args[0] {
            Value::String(path) if !line => {
                fsio::write_path(path, &text, self.options.allow_write)?;
            }
            Value::String(_) => {
                return Err(NodiaError::runtime(
                    "writeln() expects stream as first argument",
                ));
            }
            Value::Stream(stream) => self.write_stream(*stream, &text)?,
            other => {
                return Err(NodiaError::runtime(format!(
                    "{}() expects path or stream, got {}",
                    if line { "writeln" } else { "write" },
                    other.type_name()
                )));
            }
        }
        Ok(Value::Null)
    }

    pub(super) fn read_bytes_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        if args.len() == 1 {
            return match &args[0] {
                Value::String(path) => fsio::read_path_bytes(path).map(textcodec::bytes_to_value),
                Value::Stream(stream) => self
                    .read_bytes_stream(*stream)
                    .map(textcodec::bytes_to_value),
                other => Err(NodiaError::runtime(format!(
                    "read_bytes() expects path or stream, got {}",
                    other.type_name()
                ))),
            };
        }
        if args.len() == 2 {
            let stream = self.expect_stream(&args[0], "read_bytes", "first")?;
            let size = self.expect_non_negative_size(&args[1], "read_bytes", "second")?;
            return self
                .read_chunk_bytes_stream(stream, size)
                .map(textcodec::bytes_to_value);
        }
        Err(NodiaError::runtime(format!(
            "read_bytes() expects 1 or 2 argument(s), got {}",
            args.len()
        )))
    }

    pub(super) fn write_bytes_builtin(&mut self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "write_bytes")?;
        let bytes = textcodec::expect_bytes(&args[1], "write_bytes", "second")?;
        match &args[0] {
            Value::String(path) => {
                fsio::write_path_bytes(path, &bytes, self.options.allow_write)?;
            }
            Value::Stream(stream) => self.write_bytes_stream(*stream, &bytes)?,
            other => {
                return Err(NodiaError::runtime(format!(
                    "write_bytes() expects path or stream, got {}",
                    other.type_name()
                )));
            }
        }
        Ok(Value::Null)
    }

    pub(super) fn read_stream(&mut self, stream: StreamId) -> NodiaResult<String> {
        match stream {
            StreamId::Stdin => {
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
        }
    }

    pub(super) fn read_chunk_stream(
        &mut self,
        stream: StreamId,
        size: usize,
    ) -> NodiaResult<String> {
        match stream {
            StreamId::Stdin => {
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
        }
    }

    pub(super) fn read_bytes_stream(&mut self, stream: StreamId) -> NodiaResult<Vec<u8>> {
        match stream {
            StreamId::Stdin => {
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
        }
    }

    pub(super) fn read_chunk_bytes_stream(
        &mut self,
        stream: StreamId,
        size: usize,
    ) -> NodiaResult<Vec<u8>> {
        match stream {
            StreamId::Stdin => {
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
        }
    }

    pub(super) fn read_line_stream(&mut self, stream: StreamId) -> NodiaResult<Option<String>> {
        match stream {
            StreamId::Stdin => {
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
        }
    }

    pub(super) fn write_stream(&mut self, stream: StreamId, text: &str) -> NodiaResult<()> {
        match stream {
            StreamId::Stdin => Err(NodiaError::runtime("cannot write to stdin")),
            StreamId::Stdout => {
                self.output.push_str(text);
                Ok(())
            }
            StreamId::Stderr => stdio::stderr()
                .write_all(text.as_bytes())
                .map_err(|err| NodiaError::io(format!("cannot write stderr: {err}"))),
            StreamId::File(_) => self.io.borrow_mut().write(stream, text),
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
        }
    }

    pub(super) fn flush_stream(&mut self, stream: StreamId) -> NodiaResult<()> {
        match stream {
            StreamId::Stdin => Ok(()),
            StreamId::Stdout => Ok(()),
            StreamId::Stderr => stdio::stderr()
                .flush()
                .map_err(|err| NodiaError::io(format!("cannot flush stderr: {err}"))),
            StreamId::File(_) => self.io.borrow_mut().flush(stream),
        }
    }

    pub(super) fn close_stream(&mut self, stream: StreamId) -> NodiaResult<()> {
        match stream {
            StreamId::Stdin | StreamId::Stdout => Ok(()),
            StreamId::Stderr => self.flush_stream(stream),
            StreamId::File(_) => self.io.borrow_mut().close(stream),
        }
    }

    pub(super) fn eof_stream(&mut self, stream: StreamId) -> NodiaResult<bool> {
        match stream {
            StreamId::Stdin => Ok(false),
            StreamId::Stdout | StreamId::Stderr => {
                Err(NodiaError::runtime("eof() expects readable stream"))
            }
            StreamId::File(_) => self.io.borrow_mut().eof(stream),
        }
    }
}
