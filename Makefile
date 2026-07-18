autos: src/main.rs
	cargo build --release && cp target/release/autos .

self: clean autos
	./autos -c src/main.rs -o autos.ll

clean:
	cargo clean
	rm -f autos autos.ll
