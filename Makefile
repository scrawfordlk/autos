default: autos

autos: src/main.rs
	rustc src/main.rs -o autos -O

debug: clean
	rustc src/main.rs -o autos -g

self: clean autos
	./autos -c src/main.rs -o autos.ll

self-self: clean autos
	./autos -c src/main.rs -e 100 -c src/main.rs -o autos.ll

test: clean
	cd tests && cargo test

test-self-self:
	cd tests && cargo test -- --ignored

clean:
	rm -f autos autos.ll && cargo clean && cd tests/ && cargo clean
