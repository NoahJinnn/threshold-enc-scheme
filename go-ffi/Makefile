ROOT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

# PHONY means that it doesn't correspond to a file; it always runs the build commands.
.PHONY: build-dynamic
build-dynamic:
	@cd lib/ted && cargo build --release
	@cp lib/ted/target/release/libted.dylib lib/
	go build -ldflags="-r $(ROOT_DIR)lib" -o go-rust main.go 

.PHONY: run-dynamic
run-dynamic: build-dynamic
	@./go-rust

.PHONY: build-static
build-static:
	@cd lib/ted && cargo build --release
	@cp lib/ted/target/release/libted.a lib/
	go build main-static.go 

.PHONY: run-static
run-static: build-static
	@./main-static

# This is just for running the Rust lib tests natively via cargo
.PHONY: test-rust-lib
test-rust-lib:
	@cd lib/ted && cargo test -- --nocapture

.PHONY: clean
clean:
	rm -rf go-rust main_static lib/libted.so lib/ted/target