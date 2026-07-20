autos: src/main.rs
	cargo build --release && cp target/release/autos .

debug: clean
	cargo build && cp target/debug/autos .

self: clean autos
	./autos -c src/main.rs -o autos.ll


test: clean autos
	cd tests && cargo test --verbose

clean:
	cargo clean
	rm -f autos autos.ll
