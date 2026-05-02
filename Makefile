.PHONY: all release test clean run

all: release

release:
	cargo build --release

test:
	cargo test
	sh tests/smoke.sh

run:
	cargo run -- run

clean:
	cargo clean
