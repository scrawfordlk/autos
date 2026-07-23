autos: src/main.rs
	cargo build --release && cp target/release/autos .

debug: clean
	cargo build && cp target/debug/autos .

self: clean autos
	./autos -c src/main.rs -o autos.ll

self-self: clean autos
	./autos -c src/main.rs -e 100 -c src/main.rs -o autos.ll

test: clean
	cd tests && cargo test --verbose

test-self-self:
	cd tests && cargo test --verbose -- --ignored

clean:
	cargo clean
	rm -f autos autos.ll
