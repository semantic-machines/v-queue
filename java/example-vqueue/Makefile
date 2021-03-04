java_run: lib
	mvn clean install && RUST_BACKTRACE=full java -Djava.library.path=/home/itiu/target/release -jar ./target/example-vqueue-1.0-SNAPSHOT.jar

.PHONY: lib

lib:
	cargo build --release
