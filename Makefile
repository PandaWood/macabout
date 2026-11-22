.PHONY: run mock test build clean

APP := cargo run --release --

run:
	$(APP)

mock:
	$(APP) --mock

test:
	cargo test

build:
	./build.sh

clean:
	rm -rf build/ target/
