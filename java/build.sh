cd libjvqueue
cargo build --release
cd ..
sudo cp $CARGO_TARGET_DIR/release/libjvqueue.so /usr/lib/x86_64-linux-gnu/jni
cd vqueue-binding
mvn clean install
cd ..
