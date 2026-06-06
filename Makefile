.PHONY: all release test clean run bench

all: release

release:
	cargo build --release

test:
	cargo test
	sh tests/smoke.sh

run:
	cargo run -- run

bench:
	sh bench/text-workflows.sh

clean:
	cargo clean
